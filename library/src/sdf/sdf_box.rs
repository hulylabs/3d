use crate::geometry::alias::{Point, Vector};
use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_sdf_parameter, format_vector};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfBox {
    half_size: Vector,
    center: Point,
}

impl SdfBox {
    #[must_use]
    pub fn new_offset(half_size: Vector, center: Point) -> Rc<Self> {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0, "half_size must be > 0");
        Rc::new(Self { half_size, center })
    }

    #[must_use]
    pub fn new(half_size: Vector) -> Rc<Self> {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0, "half_size must be > 0");
        Self::new_offset(half_size, Point::origin())
    }
}

impl Sdf for SdfBox {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>, _level: Option<usize>) -> ShaderCode::<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = abs({parameter})-{extent}; return \
            length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);",
            parameter = format_sdf_parameter(self.center),
            extent = format_vector(self.half_size),
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
        let system_under_test = SdfBox::new(Vector::new(1.0,1.0,1.0));
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfBox::new(Vector::new(1.0,3.0,5.0));
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = "let q = abs(point)-vec3f(1.0,3.0,5.0); \
        return length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }

    #[test]
    fn test_offest_construction() {
        let half_size = Vector::new(1.0, 3.0, 5.0);
        let center = Point::new(-7.0, 13.0, -17.0);
        let system_under_test = SdfBox::new_offset(half_size, center);
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        let expected_body = "let q = abs((point-vec3f(-7.0,13.0,-17.0)))-vec3f(1.0,3.0,5.0); \
        return length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
}