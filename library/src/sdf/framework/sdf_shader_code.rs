use crate::objects::sdf_class_index::SdfClassIndex;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use crate::shader::function_name::FunctionName;
use std::fmt::Write;

pub(crate) mod sdf_conventions {
    pub(crate) const FUNCTION_NAME_SELECTION: &str = "sdf_select";
    pub(super) const PARAMETER_NAME_INDEX: &str = "sdf_index";
    pub(super) const RETURN_TYPE: &str = "f32";
}

#[must_use]
fn format_common_parameters() -> String {
    format!(
        "{parameter_point}: vec3f, {parameter_time}: f32",
        parameter_point = conventions::PARAMETER_NAME_THE_POINT,
        parameter_time = conventions::PARAMETER_NAME_THE_TIME,
    )
}

pub(crate) fn format_sdf_selection(function_to_select: &FunctionName, class_index: SdfClassIndex, buffer: &mut String) {
    writeln!(
        buffer,
        "if ({parameter_sdf_index} == {sdf_index}) {{ return {sdf_function_name}({point_parameter},{time_parameter}); }}",
        parameter_sdf_index = sdf_conventions::PARAMETER_NAME_INDEX,
        sdf_index = class_index,
        sdf_function_name = function_to_select,
        point_parameter = conventions::PARAMETER_NAME_THE_POINT,
        time_parameter = conventions::PARAMETER_NAME_THE_TIME,
    )
    .expect("failed to format sdf selection");
}

#[must_use]
pub(crate) fn format_sdf_selection_function_opening() -> String {
    format!(
        "fn {selection_function_name}({parameter_sdf_index}: i32, {common_parameters}) -> {return_type}",
        selection_function_name = sdf_conventions::FUNCTION_NAME_SELECTION,
        parameter_sdf_index = sdf_conventions::PARAMETER_NAME_INDEX,
        common_parameters = format_common_parameters(),
        return_type = sdf_conventions::RETURN_TYPE,
    )
}

#[must_use]
pub(crate) fn format_sdf_invocation(function_name: &FunctionName) -> ShaderCode<FunctionBody> {
    let code = format!(
        "return {name}({parameter_point},{parameter_time});",
        name = function_name,
        parameter_point = conventions::PARAMETER_NAME_THE_POINT,
        parameter_time = conventions::PARAMETER_NAME_THE_TIME,
    );
    ShaderCode::<FunctionBody>::new(code)
}

pub(crate) fn format_sdf_declaration(body: &ShaderCode<FunctionBody>, function_name: &FunctionName, buffer: &mut String) {
    write!(
        buffer,
        "fn {name}({common_parameters}) -> {return_type} {{\n{body}\n}}\n",
        name = function_name,
        common_parameters = format_common_parameters(),
        return_type = sdf_conventions::RETURN_TYPE,
        body = body,
    )
    .expect("failed to format sdf declaration");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::code::{FunctionBody, ShaderCode};

    #[test]
    fn test_format_sdf() {
        let function_body = ShaderCode::<FunctionBody>::new("return -7.0;".to_string());
        let function_name = FunctionName("evaluate_some_sdf".to_string());

        let mut formatted: String = String::new();
        format_sdf_declaration(&function_body, &function_name, &mut formatted);

        let expected = format!(
            "fn {function}({parameter}: vec3f, time: f32) -> f32 {{\nreturn -7.0;\n}}\n",
            function = function_name,
            parameter = conventions::PARAMETER_NAME_THE_POINT
        );
        assert_eq!(formatted, expected);
    }
}
