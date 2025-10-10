use crate::geometry::aabb::Aabb;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use std::rc::Rc;

pub(crate) struct SdfGlyph {
    aabb: Aabb
}

impl SdfGlyph {
    const RETURN_GLYPH_SAMPLE: &'static str = "return sdf_glyph_gpu_hosted_0";
    
    #[must_use]
    pub(crate) fn new(aabb: Aabb) -> Rc<Self> {
        Rc::new(Self { aabb })
    }
    
    #[must_use]
    pub(crate) fn presented_in_code(code: &String) -> bool {
        code.contains(Self::RETURN_GLYPH_SAMPLE)
    }
}


impl Sdf for SdfGlyph {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        // We use a payload to pass the glyph index through the pipeline.
        ShaderCode::<FunctionBody>::new(format!(
            "{return_glyph_sample}({payload}, {parameter});",
            return_glyph_sample = SdfGlyph::RETURN_GLYPH_SAMPLE,
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            payload = conventions::PARAMETER_NAME_PAYLOAD,
        ))
    }

    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    fn aabb(&self) -> Aabb {
        self.aabb
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::alias::Point;
    use super::*;

    #[test]
    fn test_children() {
        let system_under_test = SdfGlyph::new(Aabb::make_null());
        assert!(system_under_test.descendants().is_empty())
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfGlyph::new(Aabb::make_null());

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(7));

        let expected_body = "return sdf_glyph_gpu_hosted_0(payload, point);";
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[test]
    fn test_aabb() {
        let expected_aabb = Aabb::from_points(Point::new(1.0, 3.0, 5.0), Point::new(7.0, 13.0, 17.0));
        let system_under_test = SdfGlyph::new(expected_aabb);
        assert_eq!(expected_aabb, system_under_test.aabb());
    }
}