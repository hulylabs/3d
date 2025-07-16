use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use cgmath::Rad;
use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;

pub struct SdfSolidAngle {
    angle_sin: f64,
    angle_cos: f64,
    radius: f64,
}

impl SdfSolidAngle {
    #[must_use]
    pub fn new<Angle: Into<Rad<f64>>>(angle: Angle, radius: f64,) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        let angle_rad: Rad<f64> = angle.into();
        assert!(angle_rad.0 < FRAC_PI_2, "angle is too large");
        let (sin, cos) = angle_rad.0.sin_cos();
        Rc::new(Self { angle_sin: sin, angle_cos: cos, radius, })
    }
}

impl Sdf for SdfSolidAngle {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let sin_cos = vec2f({sin}, {cos});\n\
             let q = vec2f(length({parameter}.xz), {parameter}.y);\n\
             let l = length(q) - {radius};\n\
             let m = length(q - sin_cos*clamp(dot(q, sin_cos), 0.0, {radius}));\n\
             return max(l, m*sign({cos}*q.x-{sin}*q.y));",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            radius = format_scalar(self.radius),
            sin = format_scalar(self.angle_sin),
            cos = format_scalar(self.angle_cos),
        ))
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    fn aabb(&self) -> Aabb {
        let cap_radius = self.radius * self.angle_sin;
        
        let x_min = -cap_radius;
        let x_max = cap_radius;

        let y_min = 0.0;
        let y_max = self.radius;

        let z_min = -cap_radius;
        let z_max = cap_radius;

        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;
    use cgmath::Deg;

    #[test]
    fn test_children() {
        let system_under_test = SdfSolidAngle::new(Deg(45.0), 1.0);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let radius = 2.0;
        let system_under_test = SdfSolidAngle::new(Deg(30.0), radius);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let sin_cos = vec2f(0.5, 0.8660253882);\nlet q = vec2f(length(point.xz), point.y);\nlet l = length(q) - 2.0;\nlet m = length(q - sin_cos*clamp(dot(q, sin_cos), 0.0, 2.0));\nreturn max(l, m*sign(0.8660253882*q.x-0.5*q.y));";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}