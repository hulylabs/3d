use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfCutHollowSphere {
    radius: f64,
    cut_height: f64,
    thickness: f64,
    center: Point,
}

impl SdfCutHollowSphere {
    #[must_use]
    pub fn new_offset(radius: f64, cut_height: f64, thickness: f64, center: Point) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        assert!(cut_height.abs() < radius, "|cut_height| must be < radius");
        assert!(thickness > 0.0 && thickness < radius, "thickness must be > 0 and < radius");
        Rc::new(Self { radius, cut_height, thickness, center })
    }

    #[must_use]
    pub fn new(radius: f64, cut_height: f64, thickness: f64) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        assert!(cut_height.abs() < radius, "|h| must be < radius");
        assert!(thickness > 0.0 && thickness < radius, "thickness must be > 0 and < radius");
        Self::new_offset(radius, cut_height, thickness, Point::origin())
    }
}

impl Sdf for SdfCutHollowSphere {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let w = sqrt({radius}*{radius}-{cut_height}*{cut_height});\n\
             let q = vec2f(length({parameter}.xz), {parameter}.y);\n\
             var result: f32;\n\
             if ({cut_height}*q.x<w*q.y) {{ result = length(q-vec2f(w,{cut_height})); }}\n\
             else {{ result = abs(length(q)-{radius}); }}\n\
             return result - {thickness};",
            parameter = format_sdf_parameter(self.center),
            radius = format_scalar(self.radius),
            cut_height = format_scalar(self.cut_height),
            thickness = format_scalar(self.thickness)
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
        let system_under_test = SdfCutHollowSphere::new(3.0, 1.0, 0.2);
        assert!(system_under_test.children().is_empty())
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

    #[test]
    fn test_offset_construction() {
        let radius = 3.0;
        let cut_height = 1.0;
        let thickness = 0.2;
        let system_under_test = SdfCutHollowSphere::new_offset(radius, cut_height, thickness, Point::new(1.0, 2.0, 3.0));

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let w = sqrt(3.0*3.0-1.0*1.0);\nlet q = vec2f(length((point-vec3f(1.0,2.0,3.0)).xz), (point-vec3f(1.0,2.0,3.0)).y);\nvar result: f32;\nif (1.0*q.x<w*q.y) { result = length(q-vec2f(w,1.0)); }\nelse { result = abs(length(q)-3.0); }\nreturn result - 0.200000003;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}