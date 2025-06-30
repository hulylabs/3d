use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use cgmath::{Angle, Deg};
use std::rc::Rc;

pub struct SdfHexPrism {
    width: f64,
    height: f64,
}

impl SdfHexPrism {
    #[must_use]
    pub fn new(width: f64, height: f64) -> Rc<Self> {
        assert!(width > 0.0, "width must be positive");
        assert!(height > 0.0, "height must be positive");
        Rc::new(Self { width, height, })
    }
}

impl Sdf for SdfHexPrism {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let k: vec3f = vec3f(-0.8660254, 0.5, 0.57735);\n\
            var p = abs({parameter});\n\
            let delta = 2.0*min(dot(k.xy, p.xy), 0.0)*k.xy;\n\
            p = vec3f(p.x - delta.x, p.y - delta.y, p.z);\n\
            let d = vec2f(\n\
                length(p.xy-vec2f(clamp(p.x,-k.z*{width},k.z*{width}), {width}))*sign(p.y-{width}),\n\
                p.z-{height} );\n\
            return min(max(d.x,d.y),0.0) + length(max(d,vec2f(0.0)));",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            width = format_scalar(self.width),
            height = format_scalar(self.height),
        )) 
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let x_min = -self.width / Deg(30.0).cos();
        let x_max = -x_min;

        let y_min = -self.width;
        let y_max = self.width;

        let z_min = -self.height;
        let z_max = self.height;

        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfHexPrism::new(1.0, 2.0);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let width = 5.0;
        let height = 7.0;
        let system_under_test = SdfHexPrism::new(width, height);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let k: vec3f = vec3f(-0.8660254, 0.5, 0.57735);\nvar p = abs(point);\nlet delta = 2.0*min(dot(k.xy, p.xy), 0.0)*k.xy;\np = vec3f(p.x - delta.x, p.y - delta.y, p.z);\nlet d = vec2f(\nlength(p.xy-vec2f(clamp(p.x,-k.z*5.0,k.z*5.0), 5.0))*sign(p.y-5.0),\np.z-7.0 );\nreturn min(max(d.x,d.y),0.0) + length(max(d,vec2f(0.0)));";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}