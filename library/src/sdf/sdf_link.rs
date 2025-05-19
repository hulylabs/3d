use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfLink {
    half_length: f64,
    inner_radius: f64,
    outer_radius: f64,
    center: Point,
}

impl SdfLink {
    #[must_use]
    pub fn new_offset(half_length: f64, inner_radius: f64, outer_radius: f64, center: Point) -> Rc<Self> {
        assert!(half_length > 0.0, "length must be positive");
        assert!(inner_radius > 0.0, "inner_radius must be positive");
        assert!(outer_radius > 0.0, "outer_radius must be positive");
        Rc::new(Self { half_length, inner_radius, outer_radius, center, })
    }

    #[must_use]
    pub fn new(half_length: f64, inner_radius: f64, outer_radius: f64) -> Rc<Self> {
        assert!(half_length > 0.0, "length must be positive");
        assert!(inner_radius > 0.0, "inner_radius must be positive");
        assert!(outer_radius > 0.0, "outer_radius must be positive");
        Self::new_offset(half_length, inner_radius, outer_radius, Point::origin())
    }
}

impl Sdf for SdfLink {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = vec3f({parameter}.x, max(abs({parameter}.y)-{length},0.0), {parameter}.z);\n\
            return length(vec2f(length(q.xy)-{inner_radius},q.z)) - {outer_radius};",
            parameter = format_sdf_parameter(self.center),
            length = format_scalar(self.half_length),
            inner_radius = format_scalar(self.inner_radius),
            outer_radius = format_scalar(self.outer_radius),
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
        let system_under_test = SdfLink::new(2.0, 0.3, 0.7);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfLink::new(2.0, 0.5, 0.3);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = vec3f(point.x, max(abs(point.y)-2.0,0.0), point.z);\nreturn length(vec2f(length(q.xy)-0.5,q.z)) - 0.3000000119;";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let system_under_test = SdfLink::new_offset(1.0, 0.3, 0.7,
            Point::new(1.0, 2.0, 3.0)
        );
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = vec3f((point-vec3f(1.0,2.0,3.0)).x, max(abs((point-vec3f(1.0,2.0,3.0)).y)-1.0,0.0), (point-vec3f(1.0,2.0,3.0)).z);\nreturn length(vec2f(length(q.xy)-0.3000000119,q.z)) - 0.6999999881;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}