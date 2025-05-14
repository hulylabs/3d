use crate::geometry::alias::Point;
use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter, ShaderReadyFloat};
use crate::sdf::stack::Stack;
use cgmath::{Angle, EuclideanSpace, Rad};
use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;

pub struct SdfCone {
    angle_tan: f64,
    height: f64,
    center: Point,
}

impl SdfCone {
    #[must_use]
    pub fn new_offset<Angle: Into<Rad<f64>>>(angle: Angle, height: f64, center: Point) -> Rc<Self> {
        assert!(height > 0.0, "height must be positive");
        let angle_rad: Rad<f64> = angle.into();
        assert!(angle_rad.0 < FRAC_PI_2, "angle is too large");
        Rc::new(Self {
            angle_tan: angle_rad.tan(),
            height,
            center,
        })
    }

    #[must_use]
    pub fn new<Angle: Into<Rad<f64>>>(angle: Angle, height: f64) -> Rc<Self> {
        assert!(height > 0.0);
        Self::new_offset(angle, height, Point::origin())
    }
}

impl Sdf for SdfCone {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = {height}*vec2f({angle_tan},-1.0);\n\
            let w = vec2f(length({parameter}.xz), {parameter}.y);\n\
            let a = w - q*clamp(dot(w,q)/dot(q,q), 0.0, 1.0);\n\
            let b = w - q*vec2f(clamp(w.x/q.x, 0.0, 1.0), 1.0);\n\
            let k = sign(q.y);\n\
            let d = min(dot(a,a), dot(b,b));\n\
            let s = max(k*(w.x*q.y-w.y*q.x), k*(w.y-q.y));\n\
            return sqrt(d)*sign(s);",
            parameter = format_sdf_parameter(self.center),
            angle_tan = format_scalar(self.angle_tan),
            height = ShaderReadyFloat::new(self.height),
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
    use cgmath::Deg;

    #[test]
    fn test_children() {
        let system_under_test = SdfCone::new(Deg(30.0), 2.0);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let height = 2.0;
        let system_under_test = SdfCone::new(Deg(45.0), height);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = 2.0*vec2f(1.0,-1.0);\nlet w = vec2f(length(point.xz), point.y);\nlet a = w - q*clamp(dot(w,q)/dot(q,q), 0.0, 1.0);\nlet b = w - q*vec2f(clamp(w.x/q.x, 0.0, 1.0), 1.0);\nlet k = sign(q.y);\nlet d = min(dot(a,a), dot(b,b));\nlet s = max(k*(w.x*q.y-w.y*q.x), k*(w.y-q.y));\nreturn sqrt(d)*sign(s);";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let height = 2.0;
        let system_under_test = SdfCone::new_offset(Deg(45.0), height, Point::new(3.0, 5.0, 7.0));

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let q = 2.0*vec2f(1.0,-1.0);\nlet w = vec2f(length((point-vec3f(3.0,5.0,7.0)).xz), (point-vec3f(3.0,5.0,7.0)).y);\nlet a = w - q*clamp(dot(w,q)/dot(q,q), 0.0, 1.0);\nlet b = w - q*vec2f(clamp(w.x/q.x, 0.0, 1.0), 1.0);\nlet k = sign(q.y);\nlet d = min(dot(a,a), dot(b,b));\nlet s = max(k*(w.x*q.y-w.y*q.x), k*(w.y-q.y));\nreturn sqrt(d)*sign(s);";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
}