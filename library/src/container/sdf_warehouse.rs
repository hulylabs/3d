use crate::geometry::aabb::Aabb;
use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::framework::animation_undo_generator::AnimationUndoGenerator;
use crate::sdf::framework::code_generator::{SdfCodeGenerator, SdfRegistrator};
use crate::sdf::framework::named_sdf::UniqueSdfClassName;
use crate::sdf::framework::selection_generator::SelectionGenerator;
use std::collections::HashMap;

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
        let mut sdf_selection_uber_function = SelectionGenerator::new();
        let mut sdf_animation_undo_uber_function = AnimationUndoGenerator::new();
        
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
            sdf_selection_uber_function.add_selection(&function_to_call, index);
            sdf_animation_undo_uber_function.add_handler(sdf_class.sdf(), index);
        }
        code_generator.generate_shared_code(&mut overall_accumulated_code);
        
        overall_accumulated_code.push_str(sdf_selection_uber_function.make().as_str());
        overall_accumulated_code.push_str(sdf_animation_undo_uber_function.make().as_str());

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