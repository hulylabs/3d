use crate::geometry::alias::Point;
use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfHexPrism {
    width: f64,
    height: f64,
    center: Point,
}

impl SdfHexPrism {
    #[must_use]
    pub fn new_offset(width: f64, height: f64, center: Point) -> Rc<Self> {
        assert!(width > 0.0, "width must be positive");
        assert!(height > 0.0, "height must be positive");
        Rc::new(Self {
            width, 
            height,
            center,
        })
    }

    #[must_use]
    pub fn new(width: f64, height: f64) -> Rc<Self> {
        assert!(width > 0.0 && height > 0.0);
        Self::new_offset(width, height, Point::origin())
    }
}

impl Sdf for SdfHexPrism {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let k: vec3f = vec3f(-0.8660254, 0.5, 0.57735);\n\
            var p = abs({parameter});\n\
            let delta = 2.0*min(dot(k.xy, p.xy), 0.0)*k.xy;\n\
            p = vec3f(p.x - delta.x, p.y - delta.y, p.z);\n\
            let d = vec2f(\n\
                length(p.xy-vec2f(clamp(p.x,-k.z*{width},k.z*{width}), {width}))*sign(p.y-{width}),\n\
                p.z-{height} );\n\
            return min(max(d.x,d.y),0.0) + length(max(d,vec2f(0.0)));",
            parameter = format_sdf_parameter(self.center),
            width = format_scalar(self.width),
            height = format_scalar(self.height),
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
        let system_under_test = SdfHexPrism::new(1.0, 2.0);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let width = 5.0;
        let height = 7.0;
        let system_under_test = SdfHexPrism::new(width, height);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let k: vec3f = vec3f(-0.8660254, 0.5, 0.57735);\nvar p = abs(point);\np.xy -= 2.0*min(dot(k.xy, p.xy), 0.0)*k.xy;\nvec2f d = vec2f(\nlength(p.xy-vec2f(clamp(p.x,-k.z*5.0,k.z*5.0), 5.0))*sign(p.y-5.0),\np.z-7.0 );\nreturn min(max(d.x,d.y),0.0) + length(max(d,vec2f(0.0)));";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let width = 5.0;
        let height = 7.0;
        let system_under_test = SdfHexPrism::new_offset(width, height, Point::new(1.0, 2.0, 3.0));
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let k: vec3f = vec3f(-0.8660254, 0.5, 0.57735);\nvar p = abs((point-vec3f(1.0,2.0,3.0)));\np.xy -= 2.0*min(dot(k.xy, p.xy), 0.0)*k.xy;\nvec2f d = vec2f(\nlength(p.xy-vec2f(clamp(p.x,-k.z*5.0,k.z*5.0), 5.0))*sign(p.y-5.0),\np.z-7.0 );\nreturn min(max(d.x,d.y),0.0) + length(max(d,vec2f(0.0)));";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
}