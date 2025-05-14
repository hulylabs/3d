use crate::geometry::alias::Point;
use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode, SHADER_RETURN_KEYWORD};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::{EuclideanSpace};
use std::rc::Rc;

pub struct SdfSphere {
    radius: f64,
    center: Point,
}

impl SdfSphere {
    #[must_use]
    pub fn new_offset(radius: f64, center: Point) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        Rc::new(Self { radius, center })
    }

    #[must_use]
    pub fn new(radius: f64) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        Self::new_offset(radius, Point::origin())
    }
}

impl Sdf for SdfSphere {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "{return} length({parameter})-{radius};",
            return = SHADER_RETURN_KEYWORD,
            parameter = format_sdf_parameter(self.center),
            radius = format_scalar(self.radius)
        ))
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
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = format!("{} length({})-{:.1};", SHADER_RETURN_KEYWORD, conventions::PARAMETER_NAME_THE_POINT, expected_radius);
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
    
    #[test]
    fn test_offset_construction() {
        let expected_radius = 7.0;
        let system_under_test = SdfSphere::new_offset(expected_radius, Point::new(3.0, 5.0, -1.0));
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = format!("{} length(({}-vec3f(3.0,5.0,-1.0)))-{:.1};", SHADER_RETURN_KEYWORD, conventions::PARAMETER_NAME_THE_POINT, expected_radius);
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
}
