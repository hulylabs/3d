use crate::sdf::n_ary_operations_utils::{produce_binary_operation_body, produce_smooth_union_preparation, produce_smooth_union_return};
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::format_scalar;
use crate::sdf::stack::Stack;
use std::rc::Rc;
use crate::geometry::aabb::Aabb;

pub struct SdfUnionSmooth {
    left: Rc<dyn Sdf>,
    right: Rc<dyn Sdf>,
    smooth_size: String,
}

impl SdfUnionSmooth {
    #[must_use]
    pub fn new(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>, smooth_size: f64) -> Rc<Self> {
        assert!(smooth_size > 0.0, "smooth_size must be greater than 0");
        Rc::new(SdfUnionSmooth {
            left,
            right,
            smooth_size: format_scalar(smooth_size),
        })
    }
}

impl Sdf for SdfUnionSmooth {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);

        produce_binary_operation_body(
            children_bodies,
            level,
            |left_name, right_name| produce_smooth_union_preparation(&left_name.into(), &right_name.into(), &self.smooth_size),
            |left_name, right_name| produce_smooth_union_return(&left_name.into(), &right_name.into(), &self.smooth_size),
        )
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        Aabb::make_union(self.left.aabb(), self.right.aabb())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::n_ary_operations_utils::tests::{test_binary_operator_body_production, test_binary_operator_descendants};

    #[test]
    fn test_children() {
        test_binary_operator_descendants(|left, right| SdfUnionSmooth::new(left, right, 0.25));
    }

    #[test]
    fn test_produce_body() {
        let expected_body = "var left_0: f32;\n{\nleft_0 = ?_left;\n}\nvar right_0: f32;\n{\nright_0 = !_right;\n}\nlet h = max(0.25-abs(left_0-right_0),0.0);\nreturn min(left_0, right_0) - h*h*0.25/0.25;";
        test_binary_operator_body_production(
            |left, right| SdfUnionSmooth::new(left, right, 0.25), 
            expected_body,
        );
    }
}
