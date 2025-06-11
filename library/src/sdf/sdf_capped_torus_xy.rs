use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_point, format_scalar};
use crate::sdf::stack::Stack;
use cgmath::{Angle, Rad};
use std::rc::Rc;

pub struct SdfCappedTorusXy {
    sin: f64,
    cos: f64,
    major_radius: f64,
    minor_radius: f64,
}

impl SdfCappedTorusXy {
    #[must_use]
    pub fn new<Angle: Into<Rad<f64>>>(cut_angle: Angle, major_radius: f64, minor_radius: f64) -> Rc<Self> {
        assert!(major_radius > 0.0, "major radius must be > 0");
        assert!(minor_radius > 0.0, "minor radius must be > 0");
        assert!(minor_radius < major_radius, "doubled minor radius must be < than major");
        
        let radians = cut_angle.into();
        assert!(radians <= Rad(std::f64::consts::PI));
        
        let (sin, cos) = radians.sin_cos();
        Rc::new(Self { sin, cos, major_radius, minor_radius, })
    }
}

impl Sdf for SdfCappedTorusXy {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "var p = {parameter};\n\
            p.x = abs(p.x);\n\
            var k: f32; if ({sc}.y*p.x>{sc}.x*p.y) {{ k = dot(p.xy,{sc}.xy); }} else {{ k = length(p.xy); }};\n\
            return sqrt(dot(p,p) + {major_radius}*{major_radius} - 2.0*{major_radius}*k) - {minor_radius};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            sc = format_point(Point::new(self.sin, self.cos, 0.0)),
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

        let y_min = -total_radius;
        let y_max = total_radius;

        let z_min = -self.minor_radius;
        let z_max = self.minor_radius;
        
        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Deg;

    #[test]
    fn test_children() {
        let system_under_test = SdfCappedTorusXy::new(Deg(30.0), 2.0, 0.5);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfCappedTorusXy::new(Deg(30.0), 2.0, 0.5);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "var p = point;\np.x = abs(p.x);\nvar k: f32; if (vec3f(0.5,0.8660253882,0.0).y*p.x>vec3f(0.5,0.8660253882,0.0).x*p.y) { k = dot(p.xy,vec3f(0.5,0.8660253882,0.0).xy); } else { k = length(p.xy); };\nreturn sqrt(dot(p,p) + 2.0*2.0 - 2.0*2.0*k) - 0.5;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}