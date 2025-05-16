use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::stack::Stack;

#[must_use]
fn make_name_unique(name: &str, level: Option<usize>) -> String {
    if let Some(level) = level {
        format!("{}_{}", name, level)
    } else {
        name.to_string()
    }
}

#[must_use]
pub(super) fn produce_binary_operation_body<Operation>(children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>, operation: Operation) 
    -> ShaderCode<FunctionBody>
where
    Operation: FnOnce(&String, &String) -> String, 
{
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
        "{left} {right} return {operation};",
        left = left_sdf,
        right = right_sdf,
        operation = operation(&left_name, &right_name),
    ))
}