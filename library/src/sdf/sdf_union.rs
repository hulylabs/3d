use crate::sdf::sdf::Sdf;
use crate::sdf::shader_code::{FunctionBody, SHADER_RETURN_KEYWORD, ShaderCode};
use std::rc::Rc;
use crate::sdf::stack::Stack;

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
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        assert!(children_bodies.size() >= 2);

        let right_name = make_name_unique("right", level);
        let right_sdf = children_bodies
            .pop()
            .to_scalar_assignment(&right_name);

        let left_name = make_name_unique("left", level);
        let left_sdf = children_bodies
            .pop()
            .to_scalar_assignment(&left_name);

        ShaderCode::<FunctionBody>::new(format!(
            "{left} {right} {return} min({left_name},{right_name});",
            left = left_sdf,
            right = right_sdf,
            return = SHADER_RETURN_KEYWORD,
            left_name = left_name,
            right_name = right_name,
        ))
    }

    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.left.clone(), self.right.clone()]
    }
}

#[must_use]
fn make_name_unique(name: &str, level: Option<usize>) -> String {
    if let Some(level) = level {
        format!("{}_{}", name, level)
    } else {
        name.to_string()
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
        children_bodies.push(ShaderCode::<FunctionBody>::new(format!("{} ?_left;", SHADER_RETURN_KEYWORD)),);
        children_bodies.push(ShaderCode::<FunctionBody>::new(format!("{} !_right;", SHADER_RETURN_KEYWORD)),);

        let actual_body = system_under_test.produce_body(&mut children_bodies, Some(0));

        let expected_body = "var left_0: f32;\n { left_0 = ?_left; } var right_0: f32;\n { right_0 = !_right; } return min(left_0,right_0);";
        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}
