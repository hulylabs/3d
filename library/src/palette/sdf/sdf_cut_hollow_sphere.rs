use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use std::rc::Rc;
use crate::shader::conventions;

pub struct SdfCutHollowSphere {
    radius: f64,
    cut_height: f64,
    thickness: f64,
}

impl SdfCutHollowSphere {
    #[must_use]
    pub fn new(radius: f64, cut_height: f64, thickness: f64) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        assert!(cut_height.abs() < radius, "|cut_height| must be < radius");
        assert!(thickness > 0.0 && thickness < radius, "thickness must be > 0 and < radius");
        Rc::new(Self { radius, cut_height, thickness, })
    }
}

impl Sdf for SdfCutHollowSphere {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let w = sqrt({radius}*{radius}-{cut_height}*{cut_height});\n\
             let q = vec2f(length({parameter}.xz), {parameter}.y);\n\
             var result: f32;\n\
             if ({cut_height}*q.x<w*q.y) {{ result = length(q-vec2f(w,{cut_height})); }}\n\
             else {{ result = abs(length(q)-{radius}); }}\n\
             return result - {thickness};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            radius = format_scalar(self.radius),
            cut_height = format_scalar(self.cut_height),
            thickness = format_scalar(self.thickness),
        ))
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    fn aabb(&self) -> Aabb {
        let xz_extent = f64::max(
            self.radius, 
            (self.radius * self.radius - self.cut_height * self.cut_height).sqrt() + self.thickness,
        );
        
        let x_min = -xz_extent;
        let x_max = xz_extent;

        let y_min = -self.radius;
        let y_max = self.cut_height;

        let z_min = -xz_extent;
        let z_max = xz_extent;

        Aabb::from_points(Point::new(x_min, y_min, z_min), Point::new(x_max, y_max, z_max),).offset(self.thickness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfCutHollowSphere::new(3.0, 1.0, 0.2);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let radius = 3.0;
        let cut_height = 1.0;
        let thickness = 0.2;
        let system_under_test = SdfCutHollowSphere::new(radius, cut_height, thickness);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let w = sqrt(3.0*3.0-1.0*1.0);\nlet q = vec2f(length(point.xz), point.y);\nvar result: f32;\nif (1.0*q.x<w*q.y) { result = length(q-vec2f(w,1.0)); }\nelse { result = abs(length(q)-3.0); }\nreturn result - 0.200000003;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}