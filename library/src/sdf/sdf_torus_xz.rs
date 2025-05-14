use crate::geometry::alias::Point;
use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode, SHADER_RETURN_KEYWORD};
use crate::sdf::shader_formatting_utils::{format_point, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::{EuclideanSpace};
use std::rc::Rc;

pub struct SdfTorusXz {
    radii: Point, // using only x and y components for the torus radii (t)
    center: Point,
}

impl SdfTorusXz {
    #[must_use]
    pub fn new_offset(major_radius: f64, minor_radius: f64, center: Point) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        Rc::new(Self {
            radii: Point::new(major_radius, minor_radius, 0.0),
            center
        })
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
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = vec2f(length({parameter}.xz)-{radii}.x, {parameter}.y); \
            {return} length(q)-{radii}.y;",
            parameter = format_sdf_parameter(self.center),
            radii = format_point(Point::new(self.radii.x, self.radii.y, 0.0)),
            return = SHADER_RETURN_KEYWORD
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
        
        let expected_body = "let q = vec2f(length(point.xz)-vec3f(2.0,0.5,0.0).x, point.y); return length(q)-vec3f(2.0,0.5,0.0).y;";
        assert_eq!(expected_body, actual_body.as_str());
    }

    #[test]
    fn test_offset_construction() {
        let major_radius = 2.0;
        let minor_radius = 0.5;
        let system_under_test = SdfTorusXz::new_offset(major_radius, minor_radius, Point::new(1.0, 2.0, 3.0));
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = vec2f(length((point-vec3f(1.0,2.0,3.0)).xz)-vec3f(2.0,0.5,0.0).x, (point-vec3f(1.0,2.0,3.0)).y); return length(q)-vec3f(2.0,0.5,0.0).y;";
        assert_eq!(expected_body, actual_body.as_str());
    }
}