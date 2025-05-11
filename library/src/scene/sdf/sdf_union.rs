use crate::scene::sdf::sdf::Sdf;
use crate::scene::sdf::shader_code::{FunctionBody, SHADER_RETURN_KEYWORD, ShaderCode};
use std::rc::Rc;
use crate::scene::sdf::stack::Stack;

pub struct SdfUnion {
    left: Rc<dyn Sdf>,
    right: Rc<dyn Sdf>,
}

impl SdfUnion {
    #[must_use]
    pub fn new(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>) -> Rc<Self> {
        Rc::new(Self { left, right })
    }
}

impl Sdf for SdfUnion {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);

        let right_sdf = children_bodies
            .pop()
            .to_block_expression()
            .assign_to_variable("right");
        
        let left_sdf = children_bodies
            .pop()
            .to_block_expression()
            .assign_to_variable("left");

        ShaderCode::<FunctionBody>::new(format!(
            "{left} {right} {return} min(left,right);",
            left = left_sdf,
            right = right_sdf,
            return = SHADER_RETURN_KEYWORD
        ))
    }

    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }
}

#[cfg(test)]
mod tests {
    use crate::scene::sdf::dummy_sdf::tests::DummySdf;
    use super::*;
    
    #[test]
    fn test_children() {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::default());
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::default());
        let system_under_test = SdfUnion::new(left.clone(), right.clone());

        let children = system_under_test.children();

        assert_eq!(children.len(), 2);
        assert!(Rc::ptr_eq(&children[0], &left));
        assert!(Rc::ptr_eq(&children[1], &right));
    }

    #[test]
    fn test_produce_body() {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::new("left_17"));
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = SdfUnion::new(left.clone(), right.clone());

        let mut children_bodies = Stack::<ShaderCode::<FunctionBody>>::new();
        children_bodies.push(ShaderCode::<FunctionBody>::new(format!("{} ?_left", SHADER_RETURN_KEYWORD)),);
        children_bodies.push(ShaderCode::<FunctionBody>::new(format!("{} !_right", SHADER_RETURN_KEYWORD)),);

        let actual_body = system_under_test.produce_body(&mut children_bodies);

        let expected_body = "let left = { ?_left }; let right = { !_right }; return min(left,right);";
        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}
