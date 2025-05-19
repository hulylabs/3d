use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter, };
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfRoundCone {
    radius_major: f64,
    radius_minor: f64,
    height: f64,
    center: Point,
}

impl SdfRoundCone {
    #[must_use]
    pub fn new_offset(radius_major: f64, radius_minor: f64, height: f64, center: Point) -> Rc<Self> {
        assert!(radius_major > 0.0, "radius_major must be > 0");
        assert!(radius_minor > 0.0, "radius_minor must be > 0");
        assert!(height > 0.0, "height must be > 0");
        Rc::new(Self { radius_major, radius_minor, height, center })
    }

    #[must_use]
    pub fn new(radius_major: f64, radius_minor: f64, height: f64) -> Rc<Self> {
        assert!(radius_major > 0.0, "radius_major must be > 0");
        assert!(radius_minor > 0.0, "radius_minor must be > 0");
        assert!(height > 0.0, "height must be > 0");
        Self::new_offset(radius_major, radius_minor, height, Point::origin())
    }
}

impl Sdf for SdfRoundCone {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let b = ({radius_major}-{radius_minor})/{height};\n\
            let a = sqrt(1.0-b*b);\n\
            let q = vec2f(length({parameter}.xz), {parameter}.y);\n\
            let k = dot(q, vec2f(-b, a));\n\
            var result: f32;\n\
            if (k < 0.0) {{\n\
                result = length(q) - {radius_major};\n\
            }}\n\
            else if (k > a*{height}) {{\n\
                result = length(q-vec2f(0.0,{height})) - {radius_minor};\n\
            }}\n\
            else {{\n\
                result = dot(q, vec2f(a,b)) - {radius_major};\n\
            }}\n\
            return result;",
            parameter = format_sdf_parameter(self.center),
            radius_major = format_scalar(self.radius_major),
            radius_minor = format_scalar(self.radius_minor),
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
        let system_under_test = SdfRoundCone::new(1.0, 0.5, 2.0);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let radius_major = 1.0;
        let radius_minor = 0.5;
        let height = 2.0;
        let system_under_test = SdfRoundCone::new(radius_major, radius_minor, height);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let b = (1.0-0.5)/2.0;\nlet a = sqrt(1.0-b*b);\nlet q = vec2f(length(point.xz), point.y);\nlet k = dot(q, vec2f(-b, a));\nvar result: f32;\nif (k < 0.0) {\nresult = length(q) - 1.0;\n}\nelse if (k > a*2.0) {\nresult = length(q-vec2f(0.0,2.0)) - 0.5;\n}\nelse {\nresult = dot(q, vec2f(a,b)) - 1.0;\n}\nreturn result;";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let radius_major = 1.0;
        let radius_minor = 0.5;
        let height = 2.0;
        let system_under_test = SdfRoundCone::new_offset(radius_major, radius_minor, height, Point::new(3.0, 5.0, -1.0));

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let b = (1.0-0.5)/2.0;\nlet a = sqrt(1.0-b*b);\nlet q = vec2f(length((point-vec3f(3.0,5.0,-1.0)).xz), (point-vec3f(3.0,5.0,-1.0)).y);\nlet k = dot(q, vec2f(-b, a));\nvar result: f32;\nif (k < 0.0) {\nresult = length(q) - 1.0;\n}\nelse if (k > a*2.0) {\nresult = length(q-vec2f(0.0,2.0)) - 0.5;\n}\nelse {\nresult = dot(q, vec2f(a,b)) - 1.0;\n}\nreturn result;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}