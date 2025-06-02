use crate::geometry::aabb::Aabb;
use crate::sdf::intersection::intersection_aabb;
use crate::sdf::n_ary_operations_utils::produce_binary_operation_body;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::stack::Stack;
use std::rc::Rc;

pub struct SdfIntersection {
    left: Rc<dyn Sdf>,
    right: Rc<dyn Sdf>,
}

impl SdfIntersection {
    #[must_use]
    pub fn new(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>) -> Rc<Self> {
        Rc::new(Self { left, right })
    }
}

impl Sdf for SdfIntersection {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);

        produce_binary_operation_body(children_bodies, level
            , |_, _| "".to_string()
            , |left_name, right_name| format!("max({left_name},{right_name})"))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        intersection_aabb(self.left.clone(), self.right.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::n_ary_operations_utils::tests::{test_binary_operator_body_production, test_binary_operator_descendants};

    #[test]
    fn test_children() {
        test_binary_operator_descendants(|left, right| SdfIntersection::new(left, right));
    }

    #[test]
    fn test_produce_body() {
        let expected_body = "var left_0: f32;\n { left_0 = ?_left; } var right_0: f32;\n { right_0 = !_right; }  return max(left_0,right_0);";
        test_binary_operator_body_production(
            |left, right| SdfIntersection::new(left, right),
            expected_body,
        );
    }
}
