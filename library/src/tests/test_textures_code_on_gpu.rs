#[cfg(test)]
mod tests {
    use crate::material::procedural_textures::ProceduralTextures;
    use crate::material::texture_shader_code::procedural_texture_conventions;
    use crate::palette::material::procedural_texture_checkerboard::make_checkerboard_texture;
    use crate::serialization::pod_vector::PodVector;
    use crate::shader::function_name::FunctionName;
    use crate::tests::gpu_code_execution::tests::{execute_code, ExecutionConfig};
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction};

    #[test]
    fn test_make_checkerboard_texture() {
        let mut registrator = ProceduralTextures::new(None);
        let texture_under_test_uid = registrator.add(FunctionName("texture_checkerboard".to_string()), make_checkerboard_texture(1.0));
        let shader_code = registrator.generate_gpu_code().to_string();

        let template = ShaderFunction::new("vec4f", "vec3f", procedural_texture_conventions::FUNCTION_NAME_SELECTION)
            .with_additional_shader_code(shader_code);

        let input_points = [
            PodVector { x: 0.0, y: 0.0, z: 0.0, w: texture_under_test_uid.0 as f32, },
            PodVector { x: 0.4, y: 0.4, z: 0.4, w: texture_under_test_uid.0 as f32, },
            PodVector { x: 1.0, y: 1.0, z: 0.0, w: texture_under_test_uid.0 as f32, },
            PodVector { x: 1.6, y: 1.6, z: 0.0, w: texture_under_test_uid.0 as f32, },
            
            PodVector { x: 1.0, y: 0.0, z: 0.0, w: texture_under_test_uid.0 as f32, },
            PodVector { x: 1.1, y: 1.1, z: 1.1, w: texture_under_test_uid.0 as f32, },
            
        ];

        let expected_colors: Vec<PodVector> = vec![
            PodVector::new(0.0, 0.0, 0.0),
            PodVector::new(0.0, 0.0, 0.0),
            PodVector::new(0.0, 0.0, 0.0),
            PodVector::new(0.0, 0.0, 0.0),
            
            PodVector::new(1.0, 1.0, 1.0),
            PodVector::new(1.0, 1.0, 1.0),
        ];

        let function_execution = make_executable(&template,
            create_argument_formatter!("i32({argument}.w), vec3f({argument}.xyz), vec3f(0.0, 0.0, 0.0), 0.0"));

        let actual_colors = execute_code::<PodVector, PodVector>(&input_points, function_execution, ExecutionConfig::default());

        assert_eq!(&actual_colors, &expected_colors);
    }
}