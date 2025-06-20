use std::rc::Rc;
use crate::geometry::aabb::Aabb;
use crate::sdf::framework::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::framework::stack::Stack;

pub trait Sdf {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody>;
    
    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>>;
    
    #[must_use]
    fn aabb(&self) -> Aabb;
}
