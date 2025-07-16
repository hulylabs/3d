use crate::geometry::aabb::Aabb;
use crate::geometry::alias::{Point, Vector};
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_vector;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfBox {
    half_size: Vector,
}

impl SdfBox {
    #[must_use]
    pub fn new(half_size: Vector) -> Rc<Self> {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0, "half_size must be > 0");
        Rc::new(Self { half_size, })
    }
}

impl Sdf for SdfBox {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = abs({parameter})-{extent};\n\
            return length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            extent = format_vector(self.half_size),
        ))
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    fn aabb(&self) -> Aabb {
        Aabb::from_points(Point::from_vec(-self.half_size), Point::from_vec(self.half_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfBox::new(Vector::new(1.0,1.0,1.0));
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfBox::new(Vector::new(1.0,3.0,5.0));
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = "let q = abs(point)-vec3f(1.0,3.0,5.0);\n\
        return length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}