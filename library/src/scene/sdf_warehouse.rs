use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::code_generator::{SdfCodeGenerator, SdfRegistrator};
use crate::sdf::named_sdf::UniqueSdfClassName;
use crate::sdf::shader_code::{format_sdf_selection, format_sdf_selection_function_opening, };
use std::collections::HashMap;
use std::fmt::Write;
use crate::geometry::aabb::Aabb;

pub(crate) struct SdfWarehouse {
    properties_from_name: HashMap<UniqueSdfClassName, SdfClassIndex>,
    bounding_boxes: Vec<Aabb>,
    sdf_classes_code: String,
}

impl SdfWarehouse {
    #[must_use]
    pub(crate) fn new(sdf_classes: SdfRegistrator) -> Self {
        let mut properties_from_name: HashMap<UniqueSdfClassName, SdfClassIndex> = HashMap::new();
        let mut bounding_boxes: Vec<Aabb> = Vec::new();
        let mut shader_code = String::new();
        let mut sdf_selection_uber_function = format_sdf_selection_function_opening();
        
        let code_generator = SdfCodeGenerator::new(sdf_classes);
        for (counter, sdf_class) in code_generator.registrations().values().enumerate() {
            let index = SdfClassIndex(counter);
            let aabb = sdf_class.sdf().aabb();
            properties_from_name.insert(sdf_class.name().clone(), index);
            bounding_boxes.push(aabb);
            
            let function_to_call = code_generator.generate_unique_code_for(sdf_class, &mut shader_code);
            format_sdf_selection(&function_to_call, index, &mut sdf_selection_uber_function);
        }
        code_generator.generate_shared_code(&mut shader_code);

        write!(sdf_selection_uber_function, "return 0.0;\n}}\n").expect("failed to format sdf selection finalization");
        
        shader_code.write_str(sdf_selection_uber_function.as_str()).expect("failed to combine sdfs code and selection function");

        Self { properties_from_name, bounding_boxes, sdf_classes_code: shader_code }
    }

    #[must_use]
    pub(crate) fn properties_for_name(&self, name: &UniqueSdfClassName) -> Option<&SdfClassIndex> {
        self.properties_from_name.get(name)
    }

    #[must_use]
    pub(crate) fn aabb_from_index(&self, index: SdfClassIndex) -> &Aabb {
        assert!(index.0 < self.bounding_boxes.len());
        &self.bounding_boxes[index.0]
    }

    #[must_use]
    pub(crate) fn sdf_classes_code(&self) -> &str {
        &self.sdf_classes_code
    }
}