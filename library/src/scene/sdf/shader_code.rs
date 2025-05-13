use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::fmt::Write;
use crate::scene::sdf::shader_function_name::FunctionName;

#[derive(Clone)]
pub struct FunctionBody;
#[derive(Clone)]
pub struct VariableAssignment;
#[derive(Clone)]
pub struct Generic;

pub trait NoReturn {}
impl NoReturn for VariableAssignment {}
impl NoReturn for Generic {}

pub const SHADER_RETURN_KEYWORD: &str = "return";

#[derive(Clone)]
pub struct ShaderCode<Kind = Generic> {
    value: String,

    kind: PhantomData<Kind>,
}

impl<Kind> Hash for ShaderCode<Kind> {
    #[must_use]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<Kind> PartialEq for ShaderCode<Kind> {
    #[must_use]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<Kind> Eq for ShaderCode<Kind> {}

impl<Kind> From<ShaderCode<Kind>> for String {
    #[must_use]
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
        assert_eq!(value.matches(SHADER_RETURN_KEYWORD).count(), 1);
        Self { value, kind: PhantomData }
    }

    #[must_use]
    pub(super) fn to_scalar_assignment(&self, variable_name: &String) -> ShaderCode<VariableAssignment> {
        let evaluation = self.value.replace(SHADER_RETURN_KEYWORD, format!("{} =", variable_name).as_str());
        let assignment = format!("var {name}: f32;\n {{ {assignment} }}", name=variable_name, assignment=evaluation.trim());
        ShaderCode::<VariableAssignment>::new(assignment.to_string())
    }
}

pub(crate) mod conventions {
    pub(crate) const THE_POINT_PARAMETER_NAME: &'static str = "point";
}

#[must_use]
pub(crate) fn format_sdf_invocation(function_name: &FunctionName) -> ShaderCode::<FunctionBody> {
    let code = format!(
        "{return} {name}({parameter});",
        return = SHADER_RETURN_KEYWORD,
        name = function_name,
        parameter = conventions::THE_POINT_PARAMETER_NAME,
    );
    ShaderCode::<FunctionBody>::new(code)
}

pub(crate) fn format_sdf_declaration(body: &ShaderCode<FunctionBody>, function_name: &FunctionName, buffer: &mut String) {
    write!(
        buffer,
        "fn {name}({parameter}: vec3f) -> f32 {{ {body} }}\n",
        name = function_name,
        parameter = conventions::THE_POINT_PARAMETER_NAME,
        body = body
    )
    .expect("failed to format sdf declaration");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sdf() {
        let function_body = ShaderCode::<FunctionBody>::new("return -7.0;".to_string());
        let function_name = FunctionName("evaluate_some_sdf".to_string());

        let mut formatted: String = String::new();
        format_sdf_declaration(&function_body, &function_name, &mut formatted);

        let expected = format!(
            "fn {function}({parameter}: vec3f) -> f32 {{ return -7.0; }}\n",
            function = function_name,
            parameter = conventions::THE_POINT_PARAMETER_NAME
        );
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_function_body_conversion_to_block_expression() {
        assert_eq!(
            String::from(ShaderCode::<FunctionBody>::new("  return 13;  ".to_string()).to_scalar_assignment(&"foo".to_string())),
            String::from("var foo: f32;\n { foo = 13; }"),
        );

        assert_eq!(
            String::from(ShaderCode::<FunctionBody>::new(" return 17; ".to_string()).to_scalar_assignment(&"zig".to_string())),
            String::from("var zig: f32;\n { zig = 17; }"),
        );
    }
}
