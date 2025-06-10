use crate::objects::sdf_class_index::SdfClassIndex;
use crate::sdf::shader_function_name::FunctionName;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use crate::sdf::shader_variable_name::ShaderVariableName;

#[derive(Clone)]
pub struct FunctionBody;
#[derive(Clone)]
pub(super) struct VariableAssignment;
#[derive(Clone)]
pub struct Generic;

pub trait NoReturn {}
impl NoReturn for VariableAssignment {}
impl NoReturn for Generic {}

#[derive(Clone)]
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
        assert_eq!(value.matches("return").count(), 1);
        Self { value, kind: PhantomData }
    }

    #[must_use]
    pub(super) fn to_scalar_declaration_assignment(&self, variable_name: &ShaderVariableName) -> ShaderCode<VariableAssignment> {
        let assignment = self.make_scalar_assignment(variable_name);
        let assignment = format!("var {name}: f32;\n{assignment}", name=variable_name, assignment=assignment);
        ShaderCode::<VariableAssignment>::new(assignment.to_string())
    }

    #[must_use]
    pub(super) fn to_scalar_assignment(&self, variable_name: &ShaderVariableName) -> ShaderCode<VariableAssignment> {
        let assignment = self.make_scalar_assignment(variable_name);
        ShaderCode::<VariableAssignment>::new(assignment.to_string())
    }

    #[must_use]
    fn make_scalar_assignment(&self, variable_name: &ShaderVariableName) -> String {
        let evaluation = self.value.replace("return", format!("{} =", variable_name).as_str());
        let assignment = format!("{{\n{assignment}\n}}", assignment = evaluation.trim());
        assignment
    }
}

pub(crate) mod conventions {
    pub(crate) const PARAMETER_NAME_THE_POINT: &str = "point";
    pub(crate) const PARAMETER_NAME_SDF_INDEX: &str = "sdf_index";
    
    pub(crate) const FUNCTION_NAME_THE_SDF_SELECTION: &str = "sdf_select";
}

pub(crate) fn format_sdf_selection(function_to_select: &FunctionName, class_index: SdfClassIndex, buffer: &mut String) {
    writeln!(
        buffer,
        "if (sdf_index == {sdf_index_parameter}.0) {{ return {sdf_function_name}({point_parameter}); }}",
        sdf_index_parameter = class_index,
        sdf_function_name = function_to_select,
        point_parameter = conventions::PARAMETER_NAME_THE_POINT,
    ).expect("failed to format sdf selection");
}

#[must_use]
pub(crate) fn format_sdf_selection_function_opening() -> String {
    format!(
        "fn {selection_function_name}({parameter_sdf_index}: f32, {parameter_point}: vec3f) -> f32 {{\n",
        selection_function_name = conventions::FUNCTION_NAME_THE_SDF_SELECTION,
        parameter_sdf_index = conventions::PARAMETER_NAME_SDF_INDEX,
        parameter_point = conventions::PARAMETER_NAME_THE_POINT,
    )
}

#[must_use]
pub(crate) fn format_sdf_invocation(function_name: &FunctionName) -> ShaderCode<FunctionBody> {
    let code = format!(
        "return {name}({parameter});",
        name = function_name,
        parameter = conventions::PARAMETER_NAME_THE_POINT,
    );
    ShaderCode::<FunctionBody>::new(code)
}

pub(crate) fn format_sdf_declaration(body: &ShaderCode<FunctionBody>, function_name: &FunctionName, buffer: &mut String) {
    write!(
        buffer,
        "fn {name}({parameter}: vec3f) -> f32 {{\n{body}\n}}\n",
        name = function_name,
        parameter = conventions::PARAMETER_NAME_THE_POINT,
        body = body
    )
    .expect("failed to format sdf declaration");
}

#[cfg(test)]
impl<Kind> ShaderCode<Kind> {
    #[must_use]
    pub(crate) fn as_str(&self) -> &str {
        self.value.as_str()
    }
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
            "fn {function}({parameter}: vec3f) -> f32 {{\nreturn -7.0;\n}}\n",
            function = function_name,
            parameter = conventions::PARAMETER_NAME_THE_POINT
        );
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_function_body_conversion_to_block_expression() {
        assert_eq!(
            String::from(ShaderCode::<FunctionBody>::new("  return 13;  ".to_string()).to_scalar_declaration_assignment(&ShaderVariableName::new("foo", None))),
            String::from("var foo: f32;\n{\nfoo = 13;\n}"),
        );

        assert_eq!(
            String::from(ShaderCode::<FunctionBody>::new(" return 17; ".to_string()).to_scalar_declaration_assignment(&ShaderVariableName::new("zig", None))),
            String::from("var zig: f32;\n{\nzig = 17;\n}"),
        );
    }
}
