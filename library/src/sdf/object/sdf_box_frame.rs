use crate::geometry::aabb::Aabb;
use crate::geometry::alias::{Point, Vector};
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::{format_scalar, format_vector};
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use cgmath::EuclideanSpace;
use std::rc::Rc;
use crate::shader::conventions;

pub struct SdfBoxFrame {
    half_size: Vector,
    thickness: f64,
}

impl SdfBoxFrame {
    #[must_use]
    pub fn new(half_size: Vector, thickness: f64) -> Rc<Self> {
        assert!(thickness > 0.0, "thickness must be > 0");
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0, "half_size must be > 0");
        Rc::new(Self { half_size, thickness, })
    }
}

impl Sdf for SdfBoxFrame {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let p = abs({center})-{half_size};\n\
let q = abs(p+{thickness})-{thickness};\n\
return min(min(\n\
length(max(vec3f(p.x,q.y,q.z),vec3f(0.0)))+min(max(p.x,max(q.y,q.z)),0.0),\n\
length(max(vec3f(q.x,p.y,q.z),vec3f(0.0)))+min(max(q.x,max(p.y,q.z)),0.0)),\n\
length(max(vec3f(q.x,q.y,p.z),vec3f(0.0)))+min(max(q.x,max(q.y,p.z)),0.0));",
            center = conventions::PARAMETER_NAME_THE_POINT,
            half_size = format_vector(self.half_size),
            thickness = format_scalar(self.thickness),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        Aabb::from_points(Point::from_vec(-self.half_size), Point::from_vec(self.half_size))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_children() {
        let system_under_test = SdfBoxFrame::new(Vector::new(1.0, 1.0, 1.0), 0.1);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfBoxFrame::new(Vector::new(2.0, 3.0, 4.0), 0.1);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let p = abs(point)-vec3f(2.0,3.0,4.0);\nlet q = abs(p+0.1000000015)-0.1000000015;\nreturn min(min(\nlength(max(vec3f(p.x,q.y,q.z),vec3f(0.0)))+min(max(p.x,max(q.y,q.z)),0.0),\nlength(max(vec3f(q.x,p.y,q.z),vec3f(0.0)))+min(max(q.x,max(p.y,q.z)),0.0)),\nlength(max(vec3f(q.x,q.y,p.z),vec3f(0.0)))+min(max(q.x,max(q.y,p.z)),0.0));";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}
