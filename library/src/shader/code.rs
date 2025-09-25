use crate::shader::variable_name::VariableName;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct FunctionBody;
#[derive(Clone, Debug)]
pub(crate) struct VariableAssignment;
#[derive(Clone, Debug)]
pub struct Generic;

pub trait NoReturn {}
impl NoReturn for VariableAssignment {}
impl NoReturn for Generic {}

#[derive(Clone, Debug)]
pub struct ShaderCode<Kind = Generic> {
    value: String,

    kind: PhantomData<Kind>,
}

impl<Kind> Hash for ShaderCode<Kind> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<Kind> PartialEq for ShaderCode<Kind> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<Kind> Eq for ShaderCode<Kind> {}

impl<Kind> From<ShaderCode<Kind>> for String {
    fn from(code: ShaderCode<Kind>) -> Self {
        code.value
    }
}

impl<Kind> Display for ShaderCode<Kind> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl<Kind: NoReturn> ShaderCode<Kind> {
    #[must_use]
    pub const fn new(value: String) -> Self {
        Self { value, kind: PhantomData }
    }
}

impl ShaderCode<FunctionBody> {
    #[must_use]
    pub fn new(value: String) -> Self {
        assert_eq!(value.matches("return").count(), 1, "function body must contain exactly one return statement");
        Self { value, kind: PhantomData }
    }

    #[must_use]
    pub(crate) fn to_scalar_declaration_assignment(&self, variable_name: &VariableName) -> ShaderCode<VariableAssignment> {
        let assignment = self.make_scalar_assignment(variable_name);
        let assignment = format!("var {variable_name}: f32;\n{assignment}");
        ShaderCode::<VariableAssignment>::new(assignment.to_string())
    }

    #[must_use]
    pub(crate) fn to_scalar_assignment(&self, variable_name: &VariableName) -> ShaderCode<VariableAssignment> {
        let assignment = self.make_scalar_assignment(variable_name);
        ShaderCode::<VariableAssignment>::new(assignment.to_string())
    }

    #[must_use]
    fn make_scalar_assignment(&self, variable_name: &VariableName) -> String {
        let evaluation = self.value.replace("return", format!("{variable_name} =").as_str());
        let assignment = format!("{{\n{assignment}\n}}", assignment = evaluation.trim());
        assignment
    }
}

impl<Kind> ShaderCode<Kind> {
    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        self.value.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_body_conversion_to_block_expression() {
        assert_eq!(
            String::from(ShaderCode::<FunctionBody>::new("  return 13;  ".to_string()).to_scalar_declaration_assignment(&VariableName::new("foo", None))),
            String::from("var foo: f32;\n{\nfoo = 13;\n}"),
        );

        assert_eq!(
            String::from(ShaderCode::<FunctionBody>::new(" return 17; ".to_string()).to_scalar_declaration_assignment(&VariableName::new("zig", None))),
            String::from("var zig: f32;\n{\nzig = 17;\n}"),
        );
    }
}
