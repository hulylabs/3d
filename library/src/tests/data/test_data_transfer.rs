#[cfg(test)]
mod tests {
    use crate::material::material_properties::{MaterialClass, MaterialProperties};
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::pod_vector::PodVector;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::tests::scaffolding::gpu_state_configuration::tests::config_empty_bindings;
    use crate::tests::scaffolding::gpu_code_execution::tests::{DataBindGroupSlot, GpuCodeExecutionContext};
    use crate::tests::scaffolding::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction};

    mod material_packing_for_gpu {
        use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
        use crate::serialization::pod_vector::PodVector;
        use crate::tests::data::test_data_transfer::tests::check_material_data_probe;
        use crate::tests::scaffolding::dummy_implementations::tests::{DUMMY_IMPLEMENTATIONS, TEST_DATA_IO_BINDING_GROUP};
        use crate::tests::scaffolding::gpu_code_execution::tests::GpuCodeExecutionContext;
        use crate::tests::scaffolding::shader_entry_generator::tests::ShaderFunction;
        use test_context::test_context;

        enum FieldKind {
            Vector,
            Scalar,
        }

        #[must_use]
        fn make_shader_function(field_name_to_fetch: &str, field_kind: FieldKind) -> ShaderFunction {
            let body = match field_kind {
                FieldKind::Vector => {
                    format!("fn fetch_{field}_data(index: f32) -> vec4f {{ return vec4f(materials[u32(index)].{field}.rgb, -7.0); }}",
                            field = field_name_to_fetch)
                }
                FieldKind::Scalar => {
                    format!("fn fetch_{field}_data(index: f32) -> vec4f {{ return vec4f(f32(materials[u32(index)].{field}), 0.0, 0.0, -7.0); }}",
                            field = field_name_to_fetch)
                }
            };

            ShaderFunction::new("f32", "vec4f", format!("fetch_{field_name_to_fetch}_data"))
                .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
                .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
                .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
                .with_additional_shader_code(body)
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_albedo(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("albedo", FieldKind::Vector);
            check_material_data_probe(fixture, &template, PodVector::new_full(7.0, 2.0, 3.0, -7.0));
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_specular(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("specular", FieldKind::Vector);
            check_material_data_probe(fixture, &template, PodVector::new_full(4.0, 5.0, 6.0, -7.0));
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_specular_strength(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("specular_strength", FieldKind::Scalar);
            check_material_data_probe(fixture, &template, PodVector::new_full(1.0, 0.0, 0.0, -7.0));
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_emission(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("emission", FieldKind::Vector);
            check_material_data_probe(fixture, &template, PodVector::new_full(8.0, 9.0, 10.0, -7.0));
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_roughness(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("roughness", FieldKind::Scalar);
            check_material_data_probe(fixture, &template, PodVector::new_full(12.0, 0.0, 0.0, -7.0));
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_refractive_index_eta(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("refractive_index_eta", FieldKind::Scalar);
            check_material_data_probe(fixture, &template, PodVector::new_full(11.0, 0.0, 0.0, -7.0));
        }

        #[test_context(GpuCodeExecutionContext)]
        #[test]
        fn test_material_packing_for_gpu_material_class(fixture: &mut GpuCodeExecutionContext) {
            let template = make_shader_function("material_class", FieldKind::Scalar);
            check_material_data_probe(fixture, &template, PodVector::new_full(1.0, 0.0, 0.0, -7.0));
        }
    }

    fn check_material_data_probe(fixture: &mut GpuCodeExecutionContext, template: &ShaderFunction, expected_data: PodVector) {
        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));

        let probe = MaterialProperties::new()
            .with_albedo(7.0, 2.0, 3.0)
            .with_specular(4.0, 5.0, 6.0)
            .with_specular_strength(1.0)
            .with_emission(8.0, 9.0, 10.0)
            .with_refractive_index_eta(11.0)
            .with_roughness(12.0)
            .with_class(MaterialClass::Mirror);

        let mut serialized_materials = GpuReadySerializationBuffer::new(1, MaterialProperties::SERIALIZED_QUARTET_COUNT);
        probe.serialize_into(&mut serialized_materials);

        let mut execution_config = config_empty_bindings();
        execution_config
            .set_storage_binding_group(2, vec![], vec![
                DataBindGroupSlot::new(3, serialized_materials.backend()),
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