use crate::geometry::aabb::Aabb;
use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::framework::code_generator::{SdfCodeGenerator, SdfRegistrator};
use crate::sdf::framework::named_sdf::UniqueSdfClassName;
use crate::sdf::framework::sdf_shader_code::{format_sdf_selection, format_sdf_selection_function_opening};
use std::collections::HashMap;
use std::fmt::Write;

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
        let mut overall_accumulated_code = String::new();
        let mut sdf_selection_uber_function = format_sdf_selection_function_opening();
        writeln!(sdf_selection_uber_function, " {{").expect("failed to format brace open for sdf selection function");
        
        let code_generator = SdfCodeGenerator::new(sdf_classes);
        
        let registrations = code_generator.registrations();
        let names_ordered = {
            let mut names: Vec<_> = registrations.keys().collect();
            names.sort();
            names
        };
        
        for (name_index, name) in names_ordered.iter().enumerate() {
            let index = SdfClassIndex(name_index);
            let sdf_class = registrations.get(name).unwrap();
            
            properties_from_name.insert((*name).clone(), index);
            bounding_boxes.push(sdf_class.sdf().aabb());
            
            let function_to_call = code_generator.generate_unique_code_for(sdf_class, &mut overall_accumulated_code);
            format_sdf_selection(&function_to_call, index, &mut sdf_selection_uber_function);
        }
        code_generator.generate_shared_code(&mut overall_accumulated_code);

        write!(sdf_selection_uber_function, "return 0.0;\n}}\n").expect("failed to format sdf selection finalization");
        
        overall_accumulated_code.write_str(sdf_selection_uber_function.as_str()).expect("failed to combine sdfs code and selection function");

        Self { properties_from_name, bounding_boxes, sdf_classes_code: overall_accumulated_code }
    }

    #[must_use]
    pub(crate) fn properties_for_name(&self, name: &UniqueSdfClassName) -> Option<&SdfClassIndex> {
        self.properties_from_name.get(name)
    }
    
    #[must_use]
    pub(crate) fn name_from_index(&self, needle: SdfClassIndex) -> Option<&UniqueSdfClassName> {
        for (name, index) in self.properties_from_name.iter() {
            if *index == needle {
                return Some(name);
            }
        }
        None
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