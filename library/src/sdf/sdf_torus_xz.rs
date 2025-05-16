use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfTorusXz {
    major_radius: f64,
    minor_radius: f64,
    center: Point,
}

impl SdfTorusXz {
    #[must_use]
    pub fn new_offset(major_radius: f64, minor_radius: f64, center: Point) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        Rc::new(Self { major_radius, minor_radius, center, })
    }

    #[must_use]
    pub fn new(major_radius: f64, minor_radius: f64) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        Self::new_offset(major_radius, minor_radius, Point::origin())
    }
}

impl Sdf for SdfTorusXz {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = vec2f(length({parameter}.xz)-{major_radius}, {parameter}.y); \
            return length(q)-{minor_radius};",
            parameter = format_sdf_parameter(self.center),
            major_radius = format_scalar(self.major_radius),
            minor_radius = format_scalar(self.minor_radius),
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
        let system_under_test = SdfTorusXz::new(2.0, 0.5);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let major_radius = 2.0;
        let minor_radius = 0.5;
        let system_under_test = SdfTorusXz::new(major_radius, minor_radius);
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = "let q = vec2f(length(point.xz)-2.0, point.y); return length(q)-0.5;";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let major_radius = 2.0;
        let minor_radius = 0.5;
        let system_under_test = SdfTorusXz::new_offset(major_radius, minor_radius, Point::new(1.0, 2.0, 3.0));
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = vec2f(length((point-vec3f(1.0,2.0,3.0)).xz)-2.0, (point-vec3f(1.0,2.0,3.0)).y); return length(q)-0.5;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}