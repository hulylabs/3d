#[cfg(test)]
mod tests {
    use crate::material::procedural_textures::ProceduralTextures;
    use crate::material::texture_procedural::TextureProcedural;
    use crate::material::texture_shader_code::procedural_texture_conventions;
    use crate::serialization::pod_vector::PodVector;
    use crate::shader::code::{FunctionBody, ShaderCode};
    use crate::shader::conventions;
    use crate::shader::formatting_utils::format_scalar;
    use crate::shader::function_name::FunctionName;
    use crate::tests::gpu_code_execution::tests::{execute_code, ExecutionConfig};
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction};
    use std::fmt::Write;

    #[must_use]
    fn make_spy_texture(marker_value: f64) -> TextureProcedural {
        let code = format!("\
            return vec3f({point_parameter_name}.x, {normal_parameter_name}.y, {marker});",
            point_parameter_name = conventions::PARAMETER_NAME_THE_POINT,
            normal_parameter_name = conventions::PARAMETER_NAME_THE_NORMAL,
            marker = format_scalar(marker_value),
        );

        TextureProcedural::new(ShaderCode::<FunctionBody>::new(code.to_string()))
    }
    
    #[test]
    fn test_texture_selection_evaluation() {
        let first_marker: f64 = 0.17;
        let second_marker: f64 = 0.23;
        
        let mut registrator = ProceduralTextures::new(None);
        let texture_first = registrator.add(FunctionName("texture_a".to_string()), make_spy_texture(first_marker));
        let texture_second = registrator.add(FunctionName("texture_b".to_string()), make_spy_texture(second_marker));
        
        let template = ShaderFunction::new("vec4f", "vec3f", procedural_texture_conventions::FUNCTION_NAME_SELECTION)
            .with_additional_shader_code(registrator.generate_gpu_code().to_string());

        let input_points = [
            PodVector { x: 2.0, y: 3.0, z: 4.0, w: texture_first.0 as f32, },
            PodVector { x: 5.0, y: 6.0, z: 7.0, w: texture_second.0 as f32, },
        ];

        let expected_colors: Vec<PodVector> = vec![
            PodVector::new(2.0, 3.0, first_marker as f32),
            PodVector::new(5.0, 6.0, second_marker as f32),
        ];

        let function_execution = make_executable(&template,
    create_argument_formatter!("i32({argument}.w), vec3f({argument}.x, -3.0, -3.0), vec3f(-5.0, {argument}.y, -5.0), -7.0"));

        let actual_colors = execute_code::<PodVector, PodVector>(&input_points, function_execution, ExecutionConfig::default());
        
        assert_eq!(&actual_colors, &expected_colors);
    }
}