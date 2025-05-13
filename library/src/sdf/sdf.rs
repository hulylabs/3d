use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use std::rc::Rc;
use crate::sdf::stack::Stack;

pub trait Sdf {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode::<FunctionBody>;
    
    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>>;
}
