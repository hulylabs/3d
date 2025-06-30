use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use std::rc::Rc;
use crate::shader::conventions;

pub struct SdfTorusXz {
    major_radius: f64,
    minor_radius: f64,
}

impl SdfTorusXz {
    #[must_use]
    pub fn new(major_radius: f64, minor_radius: f64) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        Rc::new(Self { major_radius, minor_radius })
    }
}

impl Sdf for SdfTorusXz {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = vec2f(length({parameter}.xz)-{major_radius}, {parameter}.y); \
            return length(q)-{minor_radius};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            major_radius = format_scalar(self.major_radius),
            minor_radius = format_scalar(self.minor_radius),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let total_radius = self.major_radius + self.minor_radius;

        let x_min = -total_radius;
        let x_max = total_radius;

        let y_min = -self.minor_radius;
        let y_max = self.minor_radius;

        let z_min = -total_radius;
        let z_max = total_radius;

        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfTorusXz::new(2.0, 0.5);
        assert!(system_under_test.descendants().is_empty())
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
}