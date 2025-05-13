use crate::geometry::alias::{Point, Vector};
use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{conventions, FunctionBody, ShaderCode, SHADER_RETURN_KEYWORD};
use crate::sdf::shader_formatting_utils::{format_point, format_vector};
use cgmath::{EuclideanSpace, Zero};
use std::rc::Rc;
use crate::sdf::stack::Stack;

pub struct SdfBox {
    half_size: Vector,
    center: Point,
}

impl SdfBox {
    #[must_use]
    pub fn new_offset(half_size: Vector, center: Point) -> Rc<Self> {
        Rc::new(Self { half_size, center })
    }

    #[must_use]
    pub fn new(half_size: Vector) -> Rc<Self> {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0);
        Self::new_offset(half_size, Point::origin())
    }
}

impl Sdf for SdfBox {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode::<FunctionBody>>, _level: Option<usize>) -> ShaderCode::<FunctionBody> {
        const RETURN_VALUE: &str = "length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);";
        if self.center.to_vec().is_zero() {
            ShaderCode::<FunctionBody>::new(format!(
                "let q = abs({parameter})-{extent}; {return} {value}",
                parameter = conventions::PARAMETER_NAME_THE_POINT,
                extent = format_vector(self.half_size),
                return = SHADER_RETURN_KEYWORD,
                value = RETURN_VALUE,
            ))
        } else {
            ShaderCode::<FunctionBody>::new(format!(
                "let q = abs({parameter}-{center})-{extent}; {return} {value}",
                parameter = conventions::PARAMETER_NAME_THE_POINT,
                center = format_point(self.center),
                extent = format_vector(self.half_size),
                return = SHADER_RETURN_KEYWORD,
                value = RETURN_VALUE,
            ))
        }
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
        let expected_body = "let q = abs(point-vec3f(-7.0,13.0,-17.0))-vec3f(1.0,3.0,5.0); \
        return length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);";
        assert_eq!(String::from(actual_body).as_str(), expected_body);
    }
}