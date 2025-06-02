use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Vector;
use crate::sdf::n_ary_operations_utils::produce_parameter_transform_body;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::format_vector;
use crate::sdf::stack::Stack;
use std::rc::Rc;

pub struct SdfTranslation {
    translation: Vector,
    target: Rc<dyn Sdf>,
}

impl SdfTranslation {
    #[must_use]
    pub fn new(translation: Vector, target: Rc<dyn Sdf>) -> Rc<Self> {
        Rc::new(Self { translation, target })
    }
}

impl Sdf for SdfTranslation {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        produce_parameter_transform_body(children_bodies, level, || 
            format!("let {parameter} = {parameter}-{center};", 
                parameter = conventions::PARAMETER_NAME_THE_POINT, 
                center = format_vector(self.translation)
            )
        )
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.target.clone()]
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        self.target.aabb().translate(self.translation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::n_ary_operations_utils::tests::{test_unary_operator_body_production, test_unary_operator_descendants};
    use cgmath::Zero;

    #[test]
    fn test_children() {
        test_unary_operator_descendants(|descendant| SdfTranslation::new(Vector::zero(), descendant));
    }

    #[test]
    fn test_produce_body() {
        let expected_body = "var operand_0: f32;\n { let point = point-vec3f(1.0,2.0,3.0);\n { operand_0 = ?_left; } } return operand_0;";
        test_unary_operator_body_production(
            |descendant| SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), descendant),
            expected_body,
        );
    }
}