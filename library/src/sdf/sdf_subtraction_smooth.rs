use crate::sdf::binary_operations_utils::{produce_binary_operation_body, produce_smooth_union_preparation, produce_smooth_union_return};
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::format_scalar;
use crate::sdf::stack::Stack;
use std::rc::Rc;

pub struct SdfSubtractionSmooth {
    left: Rc<dyn Sdf>,
    right: Rc<dyn Sdf>,
    smooth_size: String,
}

impl SdfSubtractionSmooth {
    #[must_use]
    pub fn new(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>, smooth_size: f64) -> Rc<Self> {
        assert!(smooth_size > 0.0, "smooth_size must be greater than 0");
        Rc::new(SdfSubtractionSmooth { left, right, smooth_size: format_scalar(smooth_size) })
    }
}

impl Sdf for SdfSubtractionSmooth {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);

        produce_binary_operation_body(children_bodies, level
            , |left_name, right_name| produce_smooth_union_preparation(right_name, &format!("(-{left_name})"), &self.smooth_size)
            , |left_name, right_name| {
                let union = produce_smooth_union_return(right_name, &format!("(-{left_name})"), &self.smooth_size);
                format!("-({union})")
            }
        )
    }

    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::binary_operations_utils::tests::test_binary_operator_children;
    use crate::sdf::dummy_sdf::tests::DummySdf;

    #[test]
    fn test_children() {
        test_binary_operator_children(|left, right| SdfSubtractionSmooth::new(left, right, 0.25));
    }

    #[test]
    fn test_produce_body() {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::new("left_17"));
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = SdfSubtractionSmooth::new(left.clone(), right.clone(), 0.25);

        let mut children_bodies = Stack::<ShaderCode<FunctionBody>>::new();
        children_bodies.push(ShaderCode::<FunctionBody>::new("return ?_left;".to_string()),);
        children_bodies.push(ShaderCode::<FunctionBody>::new("return !_right;".to_string()),);

        let actual_body = system_under_test.produce_body(&mut children_bodies, Some(0));

        let expected_body = "var left_0: f32;\n { left_0 = ?_left; } var right_0: f32;\n { right_0 = !_right; } let h = max(0.25-abs(right_0-(-left_0)),0.0); return -(min(right_0, (-left_0)) - h*h*0.25/0.25);";
        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}
