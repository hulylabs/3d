use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::stack::Stack;

#[must_use]
fn make_name_unique(name: &str, level: Option<usize>) -> String {
    if let Some(level) = level { format!("{}_{}", name, level) } else { name.to_string() }
}

#[must_use]
pub(super) fn produce_binary_operation_body<Operation, Preparation>(
    children_bodies: &mut Stack<ShaderCode<FunctionBody>>,
    level: Option<usize>,
    preparation: Preparation,
    operation: Operation,
) -> ShaderCode<FunctionBody>
where
    Operation: FnOnce(&String, &String) -> String,
    Preparation: FnOnce(&String, &String) -> String,
{
    assert!(children_bodies.size() >= 2);

    let right_name = make_name_unique("right", level);
    let right_sdf = children_bodies.pop().to_scalar_assignment(&right_name);

    let left_name = make_name_unique("left", level);
    let left_sdf = children_bodies.pop().to_scalar_assignment(&left_name);

    ShaderCode::<FunctionBody>::new(format!(
        "{left} {right} {preparation} return {operation};",
        left = left_sdf,
        right = right_sdf,
        preparation = preparation(&left_name, &right_name),
        operation = operation(&left_name, &right_name),
    ))
}

#[must_use]
pub(super) fn produce_smooth_union_preparation(
    left_value: &String,
    right_value: &String,
    smooth_size: &String,
) -> String {
    format!("let h = max({smooth_size}-abs({left_value}-{right_value}),0.0);")
}

#[must_use]
pub(super) fn produce_smooth_union_return(
    left_value: &String,
    right_value: &String,
    smooth_size: &String,
) -> String {
    format!("min({left_value}, {right_value}) - h*h*0.25/{smooth_size}")
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::sdf::dummy_sdf::tests::{make_dummy_sdf, DummySdf};
    use crate::sdf::sdf_base::Sdf;
    use std::rc::Rc;
    use crate::sdf::shader_code::{FunctionBody, ShaderCode};
    use crate::sdf::stack::Stack;

    pub(crate) fn test_binary_operator_children(constructor: impl FnOnce(Rc<dyn Sdf>, Rc<dyn Sdf>) -> Rc<dyn Sdf>) {
        let left: Rc<dyn Sdf> = make_dummy_sdf();
        let right: Rc<dyn Sdf> = make_dummy_sdf();
        let system_under_test = constructor(left.clone(), right.clone());

        let children = system_under_test.children();

        assert_eq!(children.len(), 2);
        assert!(Rc::ptr_eq(&children[0], &left));
        assert!(Rc::ptr_eq(&children[1], &right));
    }

    pub(crate) fn test_binary_operator_body_production(constructor: impl FnOnce(Rc<dyn Sdf>, Rc<dyn Sdf>) -> Rc<dyn Sdf>, expected_body: &str) {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::new("left_17"));
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = constructor(left.clone(), right.clone());

        let mut children_bodies = Stack::<ShaderCode<FunctionBody>>::new();
        children_bodies.push(ShaderCode::<FunctionBody>::new("return ?_left;".to_string()),);
        children_bodies.push(ShaderCode::<FunctionBody>::new("return !_right;".to_string()),);

        let actual_body = system_under_test.produce_body(&mut children_bodies, Some(0));

        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}