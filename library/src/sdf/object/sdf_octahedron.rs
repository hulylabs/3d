use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::sdf::framework::sdf_base::Sdf;
use crate::shader::formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use std::rc::Rc;
use crate::shader::conventions;

pub struct SdfOctahedron {
    size: f64,
}

impl SdfOctahedron {
    #[must_use]
    pub fn new(size: f64) -> Rc<Self> {
        assert!(size > 0.0, "size must be > 0");
        Rc::new(Self { size, })
    }
}

impl Sdf for SdfOctahedron {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let p = abs({parameter});\n\
            let m = p.x + p.y + p.z - {size};\n\
            var q: vec3f;\n\
            var early_exit = false;\n\
            var result: f32;\n\
            if (3.0*p.x < m) {{\n\
                q = p.xyz;\n\
            }} else if (3.0*p.y < m) {{\n\
                q = p.yzx;\n\
            }} else if (3.0*p.z < m) {{\n\
                q = p.zxy;\n\
            }} else {{\n\
                early_exit = true;\
                result = m*0.57735027;\n\
            }}\n\
            if (!early_exit) {{\n\
                let k = clamp(0.5*(q.z-q.y+{size}), 0.0, {size});\n\
                result = length(vec3(q.x, q.y-{size}+k, q.z-k));\n\
            }}\n\
            return result;",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            size = format_scalar(self.size),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let size = self.size;
        Aabb::from_points(Point::new(-size, -size, -size,), Point::new(size, size, size,))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::stack::Stack;

    #[test]
    fn test_children() {
        let system_under_test = SdfOctahedron::new(1.0);
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let expected_size = 2.0;
        let system_under_test = SdfOctahedron::new(expected_size);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        let expected_body = "let p = abs(point);\nlet m = p.x + p.y + p.z - 2.0;\nvar q: vec3f;\nvar early_exit = false;\nvar result: f32;\nif (3.0*p.x < m) {\nq = p.xyz;\n} else if (3.0*p.y < m) {\nq = p.yzx;\n} else if (3.0*p.z < m) {\nq = p.zxy;\n} else {\nearly_exit = true;result = m*0.57735027;\n}\nif (!early_exit) {\nlet k = clamp(0.5*(q.z-q.y+2.0), 0.0, 2.0);\nresult = length(vec3(q.x, q.y-2.0+k, q.z-k));\n}\nreturn result;";
        assert_eq!(actual_body.as_str(), expected_body);
    }
}