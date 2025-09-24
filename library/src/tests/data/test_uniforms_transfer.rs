#[cfg(test)]
mod tests {
    use crate::geometry::alias::Point;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::uniforms::Uniforms;
    use crate::scene::camera::Camera;
    use crate::serialization::pod_vector::PodVector;
    use crate::tests::data::utils::tests::{make_shader_function, FieldKind};
    use crate::tests::scaffolding::gpu_code_execution::tests::{DataBindGroupSlot, GpuCodeExecutionContext};
    use crate::tests::scaffolding::gpu_state_configuration::tests::config_empty_bindings;
    use crate::tests::scaffolding::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction};
    use std::time::Duration;
    use cgmath::{Vector4};
    use test_context::test_context;

    const DATA_SOURCE: &'static str = "uniforms";

    #[must_use]
    fn stub_camera() -> Camera {
        Camera::new_perspective_camera(3.0, Point::new(1.0, 2.0, 3.0))
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_frame_buffer_size(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("frame_buffer_size_0", FieldKind::Vector2, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(100.0, 4.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_frame_buffer_area(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("frame_buffer_area_0", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(400.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_frame_buffer_aspect(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("frame_buffer_aspect_0", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(100.0/4.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_inverted_frame_buffer_size(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("inverted_frame_buffer_size_0", FieldKind::Vector2, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(1.0/100.0, 1.0/4.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_frame_number(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("frame_number_0", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(0.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_matrix_x(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_matrix_col_0_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().camera_space_to_world().x;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }
    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_matrix_y(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_matrix_col_1_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().camera_space_to_world().y;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }
    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_matrix_z(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_matrix_col_2_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().camera_space_to_world().z;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }
    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_matrix_w(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_matrix_col_3_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().camera_space_to_world().w;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_ray_origin_matrix_x(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_ray_origin_matrix_col_0_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().view_ray_origin().x;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }
    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_ray_origin_matrix_y(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_ray_origin_matrix_col_1_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().view_ray_origin().y;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }
    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_ray_origin_matrix_z(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_ray_origin_matrix_col_2_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().view_ray_origin().z;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }
    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_for_view_ray_origin_matrix_w(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("view_ray_origin_matrix_col_3_0", FieldKind::Vector4, DATA_SOURCE);
        let expected = stub_camera().view_ray_origin().w;
        check_uniforms_data_probe(fixture, &template, to_pod(expected));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_parallelograms_count(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("parallelograms_count_0", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(6.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_bvh_length(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("bvh_length_0", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(5.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_global_time_seconds(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("global_time_seconds_0", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(9.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_thread_grid_size_x(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("thread_grid_size_0.x", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(104.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_thread_grid_size_y(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("thread_grid_size_0.y", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(8.0, 0.0, 0.0, -7.0));
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_uniforms_packing_thread_grid_size_z(fixture: &mut GpuCodeExecutionContext) {
        let template = make_shader_function("thread_grid_size_0.z", FieldKind::Scalar, DATA_SOURCE);
        check_uniforms_data_probe(fixture, &template, PodVector::new_full(1.0, 0.0, 0.0, -7.0));
    }

    #[must_use]
    fn to_pod(expected: Vector4<f64>) -> PodVector {
        PodVector::new_full(expected.x as f32, expected.y as f32, expected.z as f32, expected.w as f32)
    }

    fn check_uniforms_data_probe(fixture: &mut GpuCodeExecutionContext, template: &ShaderFunction, expected_data: PodVector) {
        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));

        let pixel_subdivision: u32 = 8;
        let mut probe = Uniforms::new(
            FrameBufferSize::new(100, 4),
            stub_camera(),
            pixel_subdivision,
            Duration::from_secs(7)
        );
        probe.set_bvh_length(5);
        probe.set_parallelograms_count(6);
        probe.update_time(Duration::from_secs(9));
        let serialized_uniforms = probe.serialize();

        let mut execution_config = config_empty_bindings();
        execution_config
            .set_storage_binding_group(0, vec![], vec![
                DataBindGroupSlot::new(0, serialized_uniforms.backend()),
            ]);

        let test_input: [f32; 1] = [
            0.0
        ];

        let expected_output: Vec<PodVector> = vec![
            expected_data,
        ];

        let actual_output = fixture.get().execute_code::<f32, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq!(actual_output, expected_output);
    }
}