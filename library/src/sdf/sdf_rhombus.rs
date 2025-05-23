﻿use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode, };
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfRhombus {
    size_y: f64,
    size_x: f64,
    height: f64,
    corners_radius: f64,
    center: Point,
}

impl SdfRhombus {
    #[must_use]
    pub fn new_offset(size_y: f64, size_x: f64, height: f64, corners_radius: f64, center: Point) -> Rc<Self> {
        assert!(size_y > 0.0, "size_y must be > 0");
        assert!(size_x > 0.0, "size_x must be > 0");
        assert!(height > 0.0, "height must be > 0");
        assert!(corners_radius >= 0.0, "corners_radius must be >= 0");
        Rc::new(Self { size_y, size_x, height, corners_radius, center })
    }

    #[must_use]
    pub fn new(size_y: f64, size_x: f64, height: f64, corners_radius: f64, ) -> Rc<Self> {
        assert!(size_y > 0.0, "size_y must be > 0");
        assert!(size_x > 0.0, "size_x must be > 0");
        assert!(height > 0.0, "height must be > 0");
        assert!(corners_radius >= 0.0, "corners_radius must be >= 0");
        Self::new_offset(size_y, size_x, height, corners_radius, Point::origin())
    }
}

impl Sdf for SdfRhombus {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let p = abs({parameter});\n\
            let b = vec2({size_y}, {size_x});\n\
            let ndot_result = b.x*(b.x-2.0*p.x) - b.y*(b.y-2.0*p.z);\n\
            let f = clamp((ndot_result)/(dot(b,b)), -1.0, 1.0);\n\
            let q = vec2f(length(p.xz-0.5*b*vec2f(1.0-f,1.0+f))*sign(p.x*b.y+p.z*b.x-b.x*b.y)-{corners_radius}, p.y-{height});\n\
            return min(max(q.x,q.y),0.0) + length(max(q,vec2f(0.0)));",
            parameter = format_sdf_parameter(self.center),
            size_y = format_scalar(self.size_y),
            size_x = format_scalar(self.size_x),
            height = format_scalar(self.height),
            corners_radius = format_scalar(self.corners_radius),
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
        let system_under_test = SdfRhombus::new(1.0, 1.0, 1.0, 0.1);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let size_y = 1.5;
        let size_x = 2.0;
        let height = 3.0;
        let corners_radius = 0.2;
        let system_under_test = SdfRhombus::new(size_y, size_x, height, corners_radius);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let p = abs(point);\nlet b = vec2(1.5, 2.0);\nlet ndot_result = b.x*(b.x-2.0*p.x) - b.y*(b.y-2.0*p.z);\nlet f = clamp((ndot_result)/(dot(b,b)), -1.0, 1.0);\nlet q = vec2f(length(p.xz-0.5*b*vec2f(1.0-f,1.0+f))*sign(p.x*b.y+p.z*b.x-b.x*b.y)-0.200000003, p.y-3.0);\nreturn min(max(q.x,q.y),0.0) + length(max(q,vec2f(0.0)));";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let size_y = 1.5;
        let size_x = 2.0;
        let height = 3.0;
        let corners_radius = 0.2;
        let system_under_test = SdfRhombus::new_offset(
            size_y, size_x, height, corners_radius, Point::new(3.0, 5.0, -1.0)
        );

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let p = abs((point-vec3f(3.0,5.0,-1.0)));\nlet b = vec2(1.5, 2.0);\nlet ndot_result = b.x*(b.x-2.0*p.x) - b.y*(b.y-2.0*p.z);\nlet f = clamp((ndot_result)/(dot(b,b)), -1.0, 1.0);\nlet q = vec2f(length(p.xz-0.5*b*vec2f(1.0-f,1.0+f))*sign(p.x*b.y+p.z*b.x-b.x*b.y)-0.200000003, p.y-3.0);\nreturn min(max(q.x,q.y),0.0) + length(max(q,vec2f(0.0)));";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}