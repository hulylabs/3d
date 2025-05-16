use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode, };
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter, };
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfTriangularPrism {
    width: f64,
    height: f64,
    center: Point,
}

impl SdfTriangularPrism {
    #[must_use]
    pub fn new_offset(width: f64, height: f64, center: Point) -> Rc<Self> {
        assert!(width > 0.0, "width must be > 0");
        assert!(height > 0.0, "height must be > 0");
        Rc::new(Self { width, height, center, })
    }

    #[must_use]
    pub fn new(width: f64, height: f64) -> Rc<Self> {
        assert!(width > 0.0, "width must be > 0");
        assert!(height > 0.0, "height must be > 0");
        Self::new_offset(width, height, Point::origin())
    }
}

impl Sdf for SdfTriangularPrism {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let p = {parameter};\n\
            let q = abs(p);\n\
            return max(q.z-{height}, max(q.x*0.866025+p.y*0.5, -p.y)-{width}*0.5);",
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
        let system_under_test = SdfTriangularPrism::new(1.0, 2.0);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let width: f64 = 3.0;
        let height: f64 = 4.0;
        let system_under_test = SdfTriangularPrism::new(width, height);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let p = point;\nlet q = abs(p);\nreturn max(q.z-4.0, max(q.x*0.866025+p.y*0.5, -p.y)-3.0*0.5);";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let width: f64 = 3.0;
        let height: f64 = 4.0;
        let system_under_test = SdfTriangularPrism::new_offset(width, height, Point::new(1.0, 2.0, 3.0));
        
        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let p = (point-vec3f(1.0,2.0,3.0));\nlet q = abs(p);\nreturn max(q.z-4.0, max(q.x*0.866025+p.y*0.5, -p.y)-3.0*0.5);";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}