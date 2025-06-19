use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::framework::shader_formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use std::rc::Rc;

pub struct SdfCapsule {
    start: Point,
    end: Point,
    radius: f64,
}

impl SdfCapsule {
    #[must_use]
    pub fn new(start: Point, end: Point, radius: f64) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        Rc::new(Self { start, end, radius, })
    }
}

impl Sdf for SdfCapsule {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let pa = {parameter} - vec3f({a_x},{a_y},{a_z});\n\
            let ba = vec3f({b_x},{b_y},{b_z}) - vec3f({a_x},{a_y},{a_z});\n\
            let h = clamp(dot(pa,ba)/dot(ba,ba), 0.0, 1.0);\n\
            return length(pa - ba*h) - {radius};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            a_x = format_scalar(self.start.x),
            a_y = format_scalar(self.start.y),
            a_z = format_scalar(self.start.z),
            b_x = format_scalar(self.end.x),
            b_y = format_scalar(self.end.y),
            b_z = format_scalar(self.end.z),
            radius = format_scalar(self.radius)
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        Aabb::from_points(self.start, self.end).offset(self.radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;
    use cgmath::Point3;

    #[test]
    fn test_children() {
        let system_under_test = SdfCapsule::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0), 1.0);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfCapsule::new(Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0), 0.1);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let pa = point - vec3f(-1.0,0.0,0.0);\nlet ba = vec3f(1.0,0.0,0.0) - vec3f(-1.0,0.0,0.0);\nlet h = clamp(dot(pa,ba)/dot(ba,ba), 0.0, 1.0);\nreturn length(pa - ba*h) - 0.1000000015;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}