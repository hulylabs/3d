use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfSphere {
    radius: f64,
}

impl SdfSphere {
    #[must_use]
    pub fn new(radius: f64) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        Rc::new(Self { radius })
    }
}

impl Sdf for SdfSphere {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "return length({parameter})-{radius};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            radius = format_scalar(self.radius),
        ))
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    fn aabb(&self) -> Aabb {
        let offset = Point::new(self.radius, self.radius, self.radius);
        Aabb::from_points(Point::from_vec(-offset.to_vec()), offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfSphere::new(1.0);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let expected_radius = 7.0;
        let system_under_test = SdfSphere::new(expected_radius);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = format!("return length({})-{:.1};", conventions::PARAMETER_NAME_THE_POINT, expected_radius);
        assert_eq!(actual_body.as_str(), expected_body);
    }
}
