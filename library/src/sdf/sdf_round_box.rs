use crate::geometry::aabb::Aabb;
use crate::geometry::alias::{Point, Vector};
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_vector};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;

pub struct SdfRoundBox {
    half_size: Vector,
    radius: f64,
}

impl SdfRoundBox {
    #[must_use]
    pub fn new(half_size: Vector, radius: f64) -> Rc<Self> {
        assert!(radius > 0.0, "radius must be > 0");
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0, "half_size must be > 0");
        Rc::new(Self { half_size, radius })
    }
}

impl Sdf for SdfRoundBox {
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        ShaderCode::<FunctionBody>::new(format!(
            "let q = abs({parameter})-{extent} + {radius};\n\
            return length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0) - {radius};",
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            extent = format_vector(self.half_size),
            radius = format_scalar(self.radius),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        Aabb::from_points(Point::from_vec(-self.half_size), Point::from_vec(self.half_size))
    }
}