use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode, };
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfPyramid {
    height: f64,
    center: Point,
}

impl SdfPyramid {
    #[must_use]
    pub fn new_offset(height: f64, center: Point) -> Rc<Self> {
        assert!(height > 0.0, "height must be > 0");
        Rc::new(Self { height, center })
    }

    #[must_use]
    pub fn new(height: f64) -> Rc<Self> {
        assert!(height > 0.0, "height must be > 0");
        Self::new_offset(height, Point::origin())
    }
}

impl Sdf for SdfPyramid {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let h = {height};\n\
            let m2 = h*h + 0.25;\n\
            var p = {parameter};\n\
            p.x = abs(p.x);\n\
            p.z = abs(p.z);\n\
            if (p.z>p.x) {{ let temp = p.x; p.x = p.z; p.z = temp; }}\n\
            p.x -= 0.5;\n\
            p.z -= 0.5;\n\
            let q = vec3f(p.z, h*p.y - 0.5*p.x, h*p.x + 0.5*p.y);\n\
            let s = max(-q.x, 0.0);\n\
            let t = clamp((q.y-0.5*p.z)/(m2+0.25), 0.0, 1.0);\n\
            let a = m2*(q.x+s)*(q.x+s) + q.y*q.y;\n\
            let b = m2*(q.x+0.5*t)*(q.x+0.5*t) + (q.y-m2*t)*(q.y-m2*t);\n\
            var d2: f32; if (min(q.y, -q.x*m2-q.y*0.5) > 0.0) {{ d2 = 0.0; }} else {{ d2 = min(a, b); }}\n\
            return sqrt((d2+q.z*q.z)/m2) * sign(max(q.z, -p.y));",
            parameter = format_sdf_parameter(self.center),
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
        let system_under_test = SdfPyramid::new(1.0);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let height = 2.0;
        let system_under_test = SdfPyramid::new(height);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let h = 2.0;\nlet m2 = h*h + 0.25;\nvar p = point;\np.x = abs(p.x);\np.z = abs(p.z);\nif (p.z>p.x) { let temp = p.x; p.x = p.z; p.z = temp; }\np.x -= 0.5;\np.z -= 0.5;\nlet q = vec3f(p.z, h*p.y - 0.5*p.x, h*p.x + 0.5*p.y);\nlet s = max(-q.x, 0.0);\nlet t = clamp((q.y-0.5*p.z)/(m2+0.25), 0.0, 1.0);\nlet a = m2*(q.x+s)*(q.x+s) + q.y*q.y;\nlet b = m2*(q.x+0.5*t)*(q.x+0.5*t) + (q.y-m2*t)*(q.y-m2*t);\nvar d2: f32; if (min(q.y, -q.x*m2-q.y*0.5) > 0.0) { d2 = 0.0; } else { d2 = min(a, b); }\nreturn sqrt((d2+q.z*q.z)/m2) * sign(max(q.z, -p.y));";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_offset_construction() {
        let height = 2.0;
        let system_under_test = SdfPyramid::new_offset(height, Point::new(3.0, 5.0, -1.0));

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let h = 2.0;\nlet m2 = h*h + 0.25;\nvar p = (point-vec3f(3.0,5.0,-1.0));\np.x = abs(p.x);\np.z = abs(p.z);\nif (p.z>p.x) { let temp = p.x; p.x = p.z; p.z = temp; }\np.x -= 0.5;\np.z -= 0.5;\nlet q = vec3f(p.z, h*p.y - 0.5*p.x, h*p.x + 0.5*p.y);\nlet s = max(-q.x, 0.0);\nlet t = clamp((q.y-0.5*p.z)/(m2+0.25), 0.0, 1.0);\nlet a = m2*(q.x+s)*(q.x+s) + q.y*q.y;\nlet b = m2*(q.x+0.5*t)*(q.x+0.5*t) + (q.y-m2*t)*(q.y-m2*t);\nvar d2: f32; if (min(q.y, -q.x*m2-q.y*0.5) > 0.0) { d2 = 0.0; } else { d2 = min(a, b); }\nreturn sqrt((d2+q.z*q.z)/m2) * sign(max(q.z, -p.y));";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}