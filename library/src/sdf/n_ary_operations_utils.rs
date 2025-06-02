use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_variable_name::ShaderVariableName;
use crate::sdf::stack::Stack;

#[must_use]
pub(super) fn produce_binary_operation_body<Operation, Preparation>(
    children_bodies: &mut Stack<ShaderCode<FunctionBody>>,
    level: Option<usize>,
    preparation: Preparation,
    operation: Operation,
) -> ShaderCode<FunctionBody>
where
    Operation: FnOnce(&ShaderVariableName, &ShaderVariableName) -> String,
    Preparation: FnOnce(&ShaderVariableName, &ShaderVariableName) -> String,
{
    assert!(children_bodies.size() >= 2);

    let right_name = ShaderVariableName::new("right", level);
    let right_sdf = children_bodies.pop().to_scalar_declaration_assignment(&right_name);

    let left_name = ShaderVariableName::new("left", level);
    let left_sdf = children_bodies.pop().to_scalar_declaration_assignment(&left_name);

    ShaderCode::<FunctionBody>::new(format!(
        "{left} {right} {preparation} return {operation};",
        left = left_sdf,
        right = right_sdf,
        preparation = preparation(&left_name, &right_name),
        operation = operation(&left_name, &right_name),
    ))
}

#[must_use]
pub(super) fn produce_parameter_transform_body<Transform>(
    children_bodies: &mut Stack<ShaderCode<FunctionBody>>,
    level: Option<usize>,
    transform: Transform,
) -> ShaderCode<FunctionBody>
where
    Transform: FnOnce() -> String,
{
    assert!(children_bodies.size() >= 1);

    let child_name = ShaderVariableName::new("operand", level);
    let child_assignment = children_bodies.pop().to_scalar_assignment(&child_name);

    ShaderCode::<FunctionBody>::new(format!(
        "var {child_name}: f32;\n {{ {transform}\n {child_assignment} }} return {child_name};",
        transform = transform(),
        child_assignment = child_assignment,
        child_name = child_name,
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

    pub(crate) fn test_unary_operator_descendants(constructor: impl FnOnce(Rc<dyn Sdf>) -> Rc<dyn Sdf>) {
        let child: Rc<dyn Sdf> = make_dummy_sdf();
        let system_under_test = constructor(child.clone());

        let children = system_under_test.descendants();

        assert_eq!(children.len(), 1);
        assert!(Rc::ptr_eq(&children[0], &child));
    }
    
    pub(crate) fn test_binary_operator_descendants(constructor: impl FnOnce(Rc<dyn Sdf>, Rc<dyn Sdf>) -> Rc<dyn Sdf>) {
        let left: Rc<dyn Sdf> = make_dummy_sdf();
        let right: Rc<dyn Sdf> = make_dummy_sdf();
        let system_under_test = constructor(left.clone(), right.clone());

        let descendants = system_under_test.descendants();

        assert_eq!(descendants.len(), 2);
        assert!(Rc::ptr_eq(&descendants[0], &left));
        assert!(Rc::ptr_eq(&descendants[1], &right));
    }

    pub(crate) fn test_unary_operator_body_production(constructor: impl FnOnce(Rc<dyn Sdf>) -> Rc<dyn Sdf>, expected_body: &str) {
        let descendant: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = constructor(descendant);

        let mut descendant_bodies = Stack::<ShaderCode<FunctionBody>>::new();
        descendant_bodies.push(ShaderCode::<FunctionBody>::new("return ?_left;".to_string()),);

        let actual_body = system_under_test.produce_body(&mut descendant_bodies, Some(0));

        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }

    pub(crate) fn test_binary_operator_body_production(constructor: impl FnOnce(Rc<dyn Sdf>, Rc<dyn Sdf>) -> Rc<dyn Sdf>, expected_body: &str) {
        let left: Rc<dyn Sdf> = Rc::new(DummySdf::new("left_17"));
        let right: Rc<dyn Sdf> = Rc::new(DummySdf::new("right_23"));
        let system_under_test = constructor(left.clone(), right.clone());

        let mut descendant_bodies = Stack::<ShaderCode<FunctionBody>>::new();
        descendant_bodies.push(ShaderCode::<FunctionBody>::new("return ?_left;".to_string()),);
        descendant_bodies.push(ShaderCode::<FunctionBody>::new("return !_right;".to_string()),);

        let actual_body = system_under_test.produce_body(&mut descendant_bodies, Some(0));

        assert_eq!(actual_body.to_string(), expected_body.to_string());
    }
}