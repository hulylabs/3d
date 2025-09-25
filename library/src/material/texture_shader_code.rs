use crate::material::procedural_texture_index::ProceduralTextureUid;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use crate::shader::function_name::FunctionName;
use std::fmt::Write;

pub(crate) mod procedural_texture_conventions {
    pub(crate) const FUNCTION_NAME_SELECTION: &str = "procedural_texture_select";
    pub(super) const PARAMETER_NAME_INDEX: &str = "texture_index";
    pub(super) const RETURN_TYPE: &str = "vec3f";
}

#[must_use]
pub(crate) fn format_common_texture_3d_parameters() -> String {
    format!(
        "{parameter_point}: vec3f, {parameter_normal}: vec3f, {parameter_time}: f32, {parameter_dp_dx}: vec3f, {parameter_dp_dy}: vec3f",
        parameter_point = conventions::PARAMETER_NAME_THE_POINT,
        parameter_normal = conventions::PARAMETER_NAME_THE_NORMAL,
        parameter_time = conventions::PARAMETER_NAME_THE_TIME,
        parameter_dp_dx = conventions::PARAMETER_DP_DX,
        parameter_dp_dy = conventions::PARAMETER_DP_DY,
    )
}

pub(super) fn write_texture_3d_selection(function_to_select: &FunctionName, texture_index: ProceduralTextureUid, buffer: &mut String) -> anyhow::Result<()> {
    writeln!(
        buffer,
        "if ({parameter_index} == {index}) {{ return {function_name}({point_parameter},{normal_parameter},{time_parameter},{dp_dx_parameter},{dp_dy_parameter}); }}",
        parameter_index = procedural_texture_conventions::PARAMETER_NAME_INDEX,
        index = texture_index,
        function_name = function_to_select,
        point_parameter = conventions::PARAMETER_NAME_THE_POINT,
        normal_parameter = conventions::PARAMETER_NAME_THE_NORMAL,
        time_parameter = conventions::PARAMETER_NAME_THE_TIME,
        dp_dx_parameter = conventions::PARAMETER_DP_DX,
        dp_dy_parameter = conventions::PARAMETER_DP_DY,
    )?;
    Ok(())
}

pub(crate) fn write_texture_3d_selection_function_opening(buffer: &mut String) -> anyhow::Result<()> {
    writeln!(
        buffer,
        "fn {selection_function_name}({parameter_texture_index}: i32, {common_parameters}) -> {return_type} {{",
        selection_function_name = procedural_texture_conventions::FUNCTION_NAME_SELECTION,
        parameter_texture_index = procedural_texture_conventions::PARAMETER_NAME_INDEX,
        common_parameters = format_common_texture_3d_parameters(),
        return_type = procedural_texture_conventions::RETURN_TYPE,
    )?;
    Ok(())
}

pub(super) fn write_texture_3d_code(body: &ShaderCode<FunctionBody>, function_name: &FunctionName, buffer: &mut String) -> anyhow::Result<()> {
    write!(
        buffer,
        "fn {function_name}({common_parameters})->{return_type}{{\n{body}\n}}\n",
        function_name = function_name,
        common_parameters = format_common_texture_3d_parameters(),
        return_type = procedural_texture_conventions::RETURN_TYPE,
        body = body,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::procedural_texture_index::ProceduralTextureUid;
    use crate::shader::code::{FunctionBody, ShaderCode};
    use crate::shader::function_name::FunctionName;

    #[test]
    fn test_write_texture_selection() {
        let function_name = FunctionName("test_texture_function".to_string());
        let texture_index = ProceduralTextureUid(17);

        let mut buffer = "prefix: ".to_string();
        write_texture_3d_selection(&function_name, texture_index, &mut buffer).unwrap();

        assert_eq!(
            buffer,
            "prefix: if (texture_index == 17) { return test_texture_function(point,normal,time,dp_dx,dp_dy); }\n"
        )
    }

    #[test]
    fn test_write_texture_selection_function_opening() {
        let mut buffer = "prefix: ".to_string();

        write_texture_3d_selection_function_opening(&mut buffer).unwrap();

        assert_eq!(
            buffer,
            "prefix: fn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {\n"
        );
    }

    #[test]
    fn test_write_texture_code() {
        let function_name = FunctionName("perlin_noise".to_string());
        let body = ShaderCode::<FunctionBody>::new("return point;".to_string());

        let mut buffer = "prefix: ".to_string();
        write_texture_3d_code(&body, &function_name, &mut buffer).unwrap();

        assert_eq!(
            buffer,
            "prefix: fn perlin_noise(point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f)->vec3f{\nreturn point;\n}\n"
        )
    }

    #[test]
    fn test_format_common_texture_parameters() {
        let result = format_common_texture_3d_parameters();
        assert_eq!(result, "point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f");
    }
}
