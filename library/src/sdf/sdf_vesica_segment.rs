use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_point, format_scalar, format_sdf_parameter, };
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfVesicaSegment {
    width: f64,
    start: Point,
    end: Point,
    center: Point,
}

impl SdfVesicaSegment {
    #[must_use]
    pub fn new_offset(width: f64, start: Point, end: Point, center: Point) -> Rc<Self> {
        assert!(width > 0.0, "width must be > 0");
        Rc::new(Self { width, start, end, center, })
    }
    
    #[must_use]
    pub fn new(width: f64, start: Point, end: Point) -> Rc<Self> {
        assert!(width > 0.0, "width must be > 0");
        Self::new_offset(width, start, end, Point::origin())
    }
}

impl Sdf for SdfVesicaSegment {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let a = {start};\n\
            let b = {end};\n\
            let w = {width};\n\
            let c = (a+b)*0.5;\n\
            let l = length(b-a);\n\
            let v = (b-a)/l;\n\
            let y = dot({parameter}-c, v);\n\
            let q = vec2f(length({parameter}-c-y*v), abs(y));\n\
            let r = 0.5*l;\n\
            let d = 0.5*(r*r-w*w)/w;\n\
            var h: vec3f;\
            if (r*q.x<d*(q.y-r)) {{ h = vec3f(0.0,r,0.0); }} else {{ h = vec3f(-d,0.0,d+w); }}\n\
            return length(q-h.xy) - h.z;",
            parameter = format_sdf_parameter(self.center),
            start = format_point(self.start),
            end = format_point(self.end),
            width = format_scalar(self.width),
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
        let width = 1.0;
        let start = Point::new(0.0, -1.0, 0.0);
        let end = Point::new(0.0, 1.0, 0.0);
        let system_under_test = SdfVesicaSegment::new(width, start, end);
        assert!(system_under_test.children().is_empty())
    }

    #[test]
    fn test_construction() {
        let width = 0.5;
        let start = Point::new(-1.0, -2.0, -3.0);
        let end = Point::new(4.0, 5.0, 6.0);
        let system_under_test = SdfVesicaSegment::new(width, start, end);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let a = vec3f(-1.0,-2.0,-3.0);\nlet b = vec3f(4.0,5.0,6.0);\nlet w = 0.5;\nlet c = (a+b)*0.5;\nlet l = length(b-a);\nlet v = (b-a)/l;\nlet y = dot(point-c, v);\nlet q = vec2f(length(point-c-y*v), abs(y));\nlet r = 0.5*l;\nlet d = 0.5*(r*r-w*w)/w;\nvar h: vec3f;if (r*q.x<d*(q.y-r)) { h = vec3f(0.0,r,0.0); } else { h = vec3f(-d,0.0,d+w); }\nreturn length(q-h.xy) - h.z;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}