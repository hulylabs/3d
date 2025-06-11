use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::format_scalar;
use crate::sdf::stack::Stack;
use cgmath::{Angle, Deg};
use std::rc::Rc;

pub struct SdfTriangularPrism {
    width: f64,
    height: f64,
}

impl SdfTriangularPrism {
    #[must_use]
    pub fn new(width: f64, height: f64, ) -> Rc<Self> {
        assert!(width > 0.0, "width must be > 0");
        assert!(height > 0.0, "height must be > 0");
        Rc::new(Self { width, height, })
    }
}

impl Sdf for SdfTriangularPrism {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let p = {parameter};\n\
            let q = abs(p);\n\
            return max(q.z-{height}, max(q.x*0.866025+p.y*0.5, -p.y)-{width}*0.5);",
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
        /*
        Tke equilateral triangle, take it's mass center and connect it with vertices.
        Those connections will divide the triangle oto 3 smaller (isosceles) triangles.
        Base angles of it equals 30 degrees.
        */
        let (sin, cos) = Deg(30.0).sin_cos();
        
        let x_min = -self.width * cos;
        let x_max = self.width * cos;

        let y_min = -self.width * sin;
        let y_max = self.width;

        let z_min = -self.height;
        let z_max = self.height;

        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_children() {
        let system_under_test = SdfTriangularPrism::new(1.0, 2.0);
        assert!(system_under_test.descendants().is_empty())
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
}