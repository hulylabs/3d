use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::code_generator::{SdfCodeGenerator, SdfRegistrator};
use crate::sdf::named_sdf::UniqueName;
use crate::sdf::shader_code::{format_sdf_selection, format_sdf_selection_function_opening, SHADER_RETURN_KEYWORD};
use std::collections::HashMap;
use std::fmt::Write;

pub(crate) struct SdfWarehouse {
    index_from_name: HashMap<UniqueName, SdfClassIndex>,
    sdf_classes_code: String,
}

impl SdfWarehouse {
    #[must_use]
    pub(crate) fn new(sdf_classes: SdfRegistrator) -> Self {
        let mut index_from_name: HashMap<UniqueName, SdfClassIndex> = HashMap::new();
        let mut shader_code = String::new();
        let mut sdf_selection_uber_function = format_sdf_selection_function_opening();
        
        let mut counter: usize = 0;
        let code_generator = SdfCodeGenerator::new(sdf_classes);
        for sdf_class in code_generator.registrations().values() {
            let class_index = SdfClassIndex(counter);
            index_from_name.insert(sdf_class.name().clone(), class_index);
            counter += 1;
            
            let function_to_call = code_generator.generate_unique_code_for(&sdf_class, &mut shader_code);
            format_sdf_selection(&function_to_call, class_index, &mut sdf_selection_uber_function);
        }
        code_generator.generate_shared_code(&mut shader_code);

        write!(sdf_selection_uber_function, "{} 0.0;\n}}\n", SHADER_RETURN_KEYWORD).expect("failed to format sdf selection finalization");
        
        shader_code.write_str(sdf_selection_uber_function.as_str()).expect("failed to combine sdfs code and selection function");

        Self { index_from_name, sdf_classes_code: shader_code }
    }
    
    pub(crate) fn index_for_name(&self, name: &UniqueName) -> Option<&SdfClassIndex> {
        self.index_from_name.get(name)
    }

    #[must_use]
    pub(crate) fn sdf_classes_code(&self) -> &str {
        &self.sdf_classes_code
    }
}