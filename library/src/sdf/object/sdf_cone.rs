use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::{format_scalar, ShaderReadyFloat};
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use cgmath::{Angle, Rad};
use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;
use crate::shader::conventions;

pub struct SdfCone {
    angle_tan: f64,
    height: f64,
}

impl SdfCone {
    #[must_use]
    pub fn new<Angle: Into<Rad<f64>>>(angle: Angle, height: f64) -> Rc<Self> {
        assert!(height > 0.0, "height must be positive");
        let angle_rad: Rad<f64> = angle.into();
        assert!(angle_rad.0 < FRAC_PI_2, "angle is too large");
        Rc::new(Self { angle_tan: angle_rad.tan(), height, })
    }
}

impl Sdf for SdfCone {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = {height}*vec2f({angle_tan},-1.0);\n\
            let w = vec2f(length({parameter}.xz), {parameter}.y);\n\
            let a = w - q*clamp(dot(w,q)/dot(q,q), 0.0, 1.0);\n\
            let b = w - q*vec2f(clamp(w.x/q.x, 0.0, 1.0), 1.0);\n\
            let k = sign(q.y);\n\
            let d = min(dot(a,a), dot(b,b));\n\
            let s = max(k*(w.x*q.y-w.y*q.x), k*(w.y-q.y));\n\
            return sqrt(d)*sign(s);",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            angle_tan = format_scalar(self.angle_tan),
            height = ShaderReadyFloat::new(self.height),
        ))
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    fn aabb(&self) -> Aabb {
        let x_min = -self.angle_tan * self.height;
        let x_max = -x_min;

        let y_min = -self.height;
        let y_max = 0.0;

        let z_min = x_min;
        let z_max = x_max;

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
        let system_under_test = SdfCone::new(Deg(30.0), 2.0);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let height = 2.0;
        let system_under_test = SdfCone::new(Deg(45.0), height);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = 2.0*vec2f(1.0,-1.0);\nlet w = vec2f(length(point.xz), point.y);\nlet a = w - q*clamp(dot(w,q)/dot(q,q), 0.0, 1.0);\nlet b = w - q*vec2f(clamp(w.x/q.x, 0.0, 1.0), 1.0);\nlet k = sign(q.y);\nlet d = min(dot(a,a), dot(b,b));\nlet s = max(k*(w.x*q.y-w.y*q.x), k*(w.y-q.y));\nreturn sqrt(d)*sign(s);";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}