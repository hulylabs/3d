use crate::geometry::alias::Point;
use crate::scene::sdf::sdf::Sdf;
use crate::scene::sdf::shader_code::{conventions, FunctionBody, ShaderCode, SHADER_RETURN_KEYWORD};
use crate::scene::sdf::shader_formatting_utils::{format_point, ShaderReadyFloat};
use cgmath::{EuclideanSpace, Zero};
use std::rc::Rc;
use crate::scene::sdf::stack::Stack;

pub struct SdfSphere {
    radius: f64,
    center: Point,
}

impl SdfSphere {
    #[must_use]
    pub fn new_offset(radius: f64, center: Point) -> Rc<Self> {
        assert!(radius > 0.0);
        Rc::new(Self { radius, center })
    }

    #[must_use]
    pub fn new(radius: f64) -> Rc<Self> {
        assert!(radius > 0.0);
        Self::new_offset(radius, Point::origin())
    }
}

impl Sdf for SdfSphere {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>) -> ShaderCode<FunctionBody> {
        let radius = ShaderReadyFloat::new(self.radius);
        let center = format_point(self.center);

        if self.center.to_vec().is_zero() {
            ShaderCode::<FunctionBody>::new(format!(
                "{return} length({parameter})-{radius};",
                return = SHADER_RETURN_KEYWORD,
                parameter = conventions::THE_POINT_PARAMETER_NAME,
                radius = radius
            ))
        } else {
            ShaderCode::<FunctionBody>::new(format!(
                "{return} length({parameter}-{center})-{radius};",
                return = SHADER_RETURN_KEYWORD,
                parameter = conventions::THE_POINT_PARAMETER_NAME,
                center = center,
                radius = radius
            ))
        }
    }

    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_children() {
        let system_under_test = SdfSphere::new(1.0);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let expected_radius = 7.0;
        let system_under_test = SdfSphere::new(expected_radius);
        let actual_body = system_under_test.produce_body(&mut Stack::new());
        let expected_body = format!("{} length({})-{:.1};", SHADER_RETURN_KEYWORD, conventions::THE_POINT_PARAMETER_NAME, expected_radius);
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
    
    #[test]
    fn test_offset_construction() {
        let expected_radius = 7.0;
        let system_under_test = SdfSphere::new_offset(expected_radius, Point::new(3.0, 5.0, -1.0));
        let actual_body = system_under_test.produce_body(&mut Stack::new());
        let expected_body = format!("{} length({}-vec3f(3.0,5.0,-1.0))-{:.1};", SHADER_RETURN_KEYWORD, conventions::THE_POINT_PARAMETER_NAME, expected_radius);
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
}
