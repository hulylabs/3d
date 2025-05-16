use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_point, format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::{Angle, EuclideanSpace, Rad};
use std::rc::Rc;

pub struct SdfCappedTorusXy {
    sin_cos: Point, // using only x and y components for the angle/direction (sc)
    major_radius: f64,
    minor_radius: f64,
    center: Point,
}

impl SdfCappedTorusXy {
    #[must_use]
    pub fn new_offset<Angle: Into<Rad<f64>>>(cut_angle: Angle, major_radius: f64, minor_radius: f64, center: Point) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        let (sin, cos) = cut_angle.into().sin_cos();
        Rc::new(Self { sin_cos: Point::new(sin, cos, 0.0), major_radius, minor_radius, center, })
    }

    #[must_use]
    pub fn new<Angle: Into<Rad<f64>>>(cut_angle: Angle, major_radius: f64, minor_radius: f64) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        Self::new_offset(cut_angle, major_radius, minor_radius, Point::origin())
    }
}

impl Sdf for SdfCappedTorusXy {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "var p = {parameter};\n\
            p.x = abs(p.x);\n\
            var k: f32; if ({sc}.y*p.x>{sc}.x*p.y)  {{ k = dot(p.xy,{sc}.xy); }} else {{ k = length(p.xy); }};\n\
            return sqrt(dot(p,p) + {major_radius}*{major_radius} - 2.0*{major_radius}*k) - {minor_radius};",
            parameter = format_sdf_parameter(self.center),
            sc = format_point(Point::new(self.sin_cos.x, self.sin_cos.y, 0.0)),
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
    use cgmath::Deg;

    #[test]
    fn test_children() {
        let system_under_test = SdfCappedTorusXy::new(Deg(30.0), 2.0, 0.5);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfCappedTorusXy::new(Deg(30.0), 2.0, 0.5);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "var p = point;\np.x = abs(p.x);\nvar k: f32; if (vec3f(0.5,0.8660253882,0.0).y*p.x>vec3f(0.5,0.8660253882,0.0).x*p.y)  { k = dot(p.xy,vec3f(0.5,0.8660253882,0.0).xy); } else { k = length(p.xy); };\nreturn sqrt(dot(p,p) + 2.0*2.0 - 2.0*2.0*k) - 0.5;";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let system_under_test = SdfCappedTorusXy::new_offset(Deg(30.0), 2.0, 0.5, Point::new(1.0, 2.0, 3.0));
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "var p = (point-vec3f(1.0,2.0,3.0));\np.x = abs(p.x);\nvar k: f32; if (vec3f(0.5,0.8660253882,0.0).y*p.x>vec3f(0.5,0.8660253882,0.0).x*p.y)  { k = dot(p.xy,vec3f(0.5,0.8660253882,0.0).xy); } else { k = length(p.xy); };\nreturn sqrt(dot(p,p) + 2.0*2.0 - 2.0*2.0*k) - 0.5;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}