use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::framework::sdf_shader_code::{format_sdf_selection, format_sdf_selection_function_opening};
use crate::shader::function_name::FunctionName;

pub(crate) struct SelectionGenerator {
    sdf_selection_uber_function: String,
}

impl SelectionGenerator {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            sdf_selection_uber_function: format_sdf_selection_function_opening(),
        }
    }

    pub(crate) fn add_selection(&mut self, function_to_select: &FunctionName, selection_index: SdfClassIndex) {
        format_sdf_selection(function_to_select, selection_index, &mut self.sdf_selection_uber_function);
    }

    #[must_use]
    pub(crate) fn make(mut self) -> String {
        self.sdf_selection_uber_function.push_str("return 0.0;\n}\n");
        self.sdf_selection_uber_function
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_additions() {
        let system_under_test = SelectionGenerator::new();
        let empty_selection = system_under_test.make();
        assert_eq!(empty_selection, "fn sdf_select(sdf_index: i32, point: vec3f, time: f32) -> f32 {\nreturn 0.0;\n}\n");
    }

    #[test]
    fn test_several_additions() {
        let mut system_under_test = SelectionGenerator::new();
        system_under_test.add_selection(&FunctionName("a".to_string()), SdfClassIndex(7));
        system_under_test.add_selection(&FunctionName("b".to_string()), SdfClassIndex(5));
        let empty_selection = system_under_test.make();
        assert_eq!(empty_selection, "fn sdf_select(sdf_index: i32, point: vec3f, time: f32) -> f32 {\nif (sdf_index == 7) { return a(point,time); }\nif (sdf_index == 5) { return b(point,time); }\nreturn 0.0;\n}\n");
    }
}