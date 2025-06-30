use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use std::rc::Rc;
use crate::shader::conventions;

pub struct SdfLink {
    half_length: f64,
    inner_radius: f64,
    outer_radius: f64,
}

impl SdfLink {
    #[must_use]
    pub fn new(half_length: f64, inner_radius: f64, outer_radius: f64, ) -> Rc<Self> {
        assert!(half_length > 0.0, "length must be positive");
        assert!(inner_radius > 0.0, "inner_radius must be positive");
        assert!(outer_radius > 0.0, "outer_radius must be positive");
        Rc::new(Self { half_length, inner_radius, outer_radius, })
    }
}

impl Sdf for SdfLink {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = vec3f({parameter}.x, max(abs({parameter}.y)-{length},0.0), {parameter}.z);\n\
            return length(vec2f(length(q.xy)-{inner_radius},q.z)) - {outer_radius};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            length = format_scalar(self.half_length),
            inner_radius = format_scalar(self.inner_radius),
            outer_radius = format_scalar(self.outer_radius),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let enlargement = self.outer_radius + self.inner_radius;
        
        let x_min = -enlargement;
        let x_max = enlargement;

        let y_min = -self.half_length - enlargement;
        let y_max = self.half_length + enlargement;

        let z_min = -self.outer_radius;
        let z_max = self.outer_radius;

        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfLink::new(2.0, 0.3, 0.7);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfLink::new(2.0, 0.5, 0.3);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = vec3f(point.x, max(abs(point.y)-2.0,0.0), point.z);\nreturn length(vec2f(length(q.xy)-0.5,q.z)) - 0.3000000119;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}