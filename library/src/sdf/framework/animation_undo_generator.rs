use std::rc::Rc;
use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::sdf_shader_code::{format_sdf_animation_application, format_sdf_animation_undo_function_opening};
use crate::shader::conventions;

pub(crate) struct AnimationUndoGenerator {
    animation_application_uber_function: String,
}

impl AnimationUndoGenerator {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            animation_application_uber_function: format_sdf_animation_undo_function_opening(),
        }
    }
    
    pub(crate) fn add_handler(&mut self, sdf: Rc<dyn Sdf>, selection_index: SdfClassIndex) {
        if let Some(animation_application_code) = sdf.animation_only() {
            format_sdf_animation_application(selection_index, animation_application_code, &mut self.animation_application_uber_function);
        }
    }

    #[must_use]
    pub(crate) fn make(mut self) -> String {
        let default_code_branch = format!("return {};\n}}\n", conventions::PARAMETER_NAME_THE_POINT);
        self.animation_application_uber_function.push_str(default_code_branch.as_str());
        self.animation_application_uber_function
    }
}
