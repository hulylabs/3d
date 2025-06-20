use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Vector;
use std::rc::Rc;
use crate::sdf::framework::n_ary_operations_utils::produce_parameter_transform_body;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::framework::shader_formatting_utils::format_vector;
use crate::sdf::framework::stack::Stack;

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
    use cgmath::Zero;
    use crate::sdf::framework::n_ary_operations_utils::tests::{test_unary_operator_body_production, test_unary_operator_descendants};

    #[test]
    fn test_children() {
        test_unary_operator_descendants(|descendant| SdfTranslation::new(Vector::zero(), descendant));
    }

    #[test]
    fn test_produce_body() {
        let expected_body = "var operand_0: f32;\n{\nlet point = point-vec3f(1.0,2.0,3.0);\n{\noperand_0 = ?_left;\n}\n}\nreturn operand_0;";
        test_unary_operator_body_production(
            |descendant| SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), descendant),
            expected_body,
        );
    }
}