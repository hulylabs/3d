use crate::geometry::aabb::Aabb;
use crate::sdf::composition::intersection::intersection_aabb;
use crate::sdf::framework::n_ary_operations_utils::{produce_binary_operation_body, produce_smooth_union_preparation, produce_smooth_union_return};
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use std::rc::Rc;
use crate::shader::code::{FunctionBody, ShaderCode};

pub struct SdfIntersectionSmooth {
    left: Rc<dyn Sdf>,
    right: Rc<dyn Sdf>,
    smooth_size: String,
}

impl SdfIntersectionSmooth {
    #[must_use]
    pub fn new(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>, smooth_size: f64) -> Rc<Self> {
        assert!(smooth_size > 0.0, "smooth_size must be greater than 0");
        Rc::new(SdfIntersectionSmooth { left, right, smooth_size: format_scalar(smooth_size) })
    }
}

impl Sdf for SdfIntersectionSmooth {
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);
        
        produce_binary_operation_body(children_bodies, level
            , |left_name, right_name| produce_smooth_union_preparation(&format!("(-{left_name})"), &format!("(-{right_name})"), &self.smooth_size)
            , |left_name, right_name| {
                let union = produce_smooth_union_return(&format!("(-{left_name})"), &format!("(-{right_name})"), &self.smooth_size);
                format!("-({union})")
            }
        )
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }

    fn aabb(&self) -> Aabb {
        intersection_aabb(self.left.clone(), self.right.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::dummy_sdf::tests::DummySdf;
    use crate::sdf::framework::n_ary_operations_utils::tests::test_binary_operator_descendants;
    use crate::sdf::framework::sdf_base::Sdf;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        test_binary_operator_descendants(|left, right| SdfIntersectionSmooth::new(left, right, 0.25));
    }

    #[test]
    fn test_produce_body() {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::new("left_17"));
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = SdfIntersectionSmooth::new(left.clone(), right.clone(), 0.25);

        let mut children_bodies = Stack::<ShaderCode<FunctionBody>>::new();
        children_bodies.push(ShaderCode::<FunctionBody>::new("return ?_left;".to_string()),);
        children_bodies.push(ShaderCode::<FunctionBody>::new("return !_right;".to_string()),);

        let actual_body = system_under_test.produce_body(&mut children_bodies, Some(0));

        let expected_body = "var left_0: f32;\n{\nleft_0 = ?_left;\n}\nvar right_0: f32;\n{\nright_0 = !_right;\n}\nlet h = max(0.25-abs((-left_0)-(-right_0)),0.0);\nreturn -(min((-left_0), (-right_0)) - h*h*0.25/0.25);";
        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}
