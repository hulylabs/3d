use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use std::rc::Rc;
use crate::sdf::binary_operations_utils::produce_binary_operation_body;
use crate::sdf::stack::Stack;

pub struct SdfIntersection {
    left: Rc<dyn Sdf>,
    right: Rc<dyn Sdf>,
}

impl SdfIntersection {
    #[must_use]
    pub fn new(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>) -> Rc<Self> {
        Rc::new(Self { left, right })
    }
}

impl Sdf for SdfIntersection {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);

        produce_binary_operation_body(children_bodies, level
          , |left_name, right_name| format!("max({left_name},{right_name})"))
    }

    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }
}

#[cfg(test)]
mod tests {
    use crate::sdf::dummy_sdf::tests::{make_dummy_sdf, DummySdf};
    use super::*;
    
    #[test]
    fn test_children() {
        let left: Rc<dyn Sdf> = make_dummy_sdf();
        let right: Rc<dyn Sdf> = make_dummy_sdf();
        let system_under_test = SdfIntersection::new(left.clone(), right.clone());

        let children = system_under_test.children();

        assert_eq!(children.len(), 2);
        assert!(Rc::ptr_eq(&children[0], &left));
        assert!(Rc::ptr_eq(&children[1], &right));
    }

    #[test]
    fn test_produce_body() {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::new("left_17"));
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = SdfIntersection::new(left.clone(), right.clone());

        let mut children_bodies = Stack::<ShaderCode<FunctionBody>>::new();
        children_bodies.push(ShaderCode::<FunctionBody>::new("return ?_left;".to_string()),);
        children_bodies.push(ShaderCode::<FunctionBody>::new("return !_right;".to_string()),);

        let actual_body = system_under_test.produce_body(&mut children_bodies, Some(0));

        let expected_body = "var left_0: f32;\n { left_0 = ?_left; } var right_0: f32;\n { right_0 = !_right; } return max(left_0,right_0);";
        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}
