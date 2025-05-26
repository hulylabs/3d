#[cfg(test)]
mod tests {
    use std::f32::consts::SQRT_2;
    use crate::geometry::alias::Vector;
    use crate::geometry::transform::Affine;
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
    use crate::gpu::resources::ComputeRoutineEntryPoint;
    use crate::objects::common_properties::Linkage;
    use crate::objects::material_index::MaterialIndex;
    use crate::objects::sdf::SdfInstance;
    use crate::objects::sdf_class_index::SdfClassIndex;
    use crate::scene::sdf_warehouse::SdfWarehouse;
    use crate::sdf::code_generator::SdfRegistrator;
    use crate::sdf::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::sdf_box::SdfBox;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::pod_vector::PodVector;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::tests::assert_utils::tests::assert_eq;
    use crate::tests::common::tests::COMMON_GPU_EVALUATIONS_EPSILON;
    use crate::tests::gpu_code_execution::tests::{execute_code, BindGroupSlot, ExecutionConfig};
    use crate::utils::object_uid::ObjectUid;
    use bytemuck::{Pod, Zeroable};
    use std::fmt::Write;
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction, TypeDeclaration};

    #[repr(C)]
    #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
    struct PositionAndDirection {
        position: PodVector,
        direction: PodVector,
    }

    const DATA_BINDING_GROUP: u32 = 3;
    
    impl ShaderFunction {
        #[must_use]
        fn with_position_and_direction_type(self: ShaderFunction) -> ShaderFunction {
            self.with_custom_type(
                TypeDeclaration::new("PositionAndDirection", "position", "vec4f")
                    .with_field("direction", "vec4f"))
        }
    }
    
    #[test]
    fn test_perfect_reflection_evaluation() {
        /* The main objective here is to test the 'evaluate_reflection' function with the
        zero roughness passed to it.*/
        
        let template = ShaderFunction::new("PositionAndDirection", "vec4f", "evaluate_reflection_t")
            .with_position_and_direction_type()
            .with_binding_group(DATA_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code("fn sdf_select(value: f32, vector: vec3f) -> f32 { return 0.0; }")
            .with_additional_shader_code(
            r#"fn evaluate_reflection_t(incident: vec3f, normal: vec3f) -> vec4f 
                { return vec4f(evaluate_reflection(incident, normal, vec3f(0.0), 0.0), 0.0); }"#
            );

        let function_execution = make_executable(&template, 
            create_argument_formatter!("{argument}.position.xyz, {argument}.direction.xyz"));

        let execution_config = {
            let mut ware = ExecutionConfig::new();
            ware
                .set_data_binding_group(DATA_BINDING_GROUP)
                .set_entry_point(ComputeRoutineEntryPoint::TestDefault)
                .add_dummy_binding_group(0, vec![])
                .add_dummy_binding_group(1, vec![])
                .add_dummy_binding_group(2, vec![])
            ;
            ware
        };

        let test_input = [
            PositionAndDirection {
                position: PodVector::new(0.0, 1.0, 0.0), direction: PodVector::new(0.0, 1.0, 0.0),
            },
            PositionAndDirection {
                position: PodVector::new(SQRT_2, SQRT_2, 0.0), direction: PodVector::new(0.0, 1.0, 0.0),
            },
        ];

        // xyz: shifted position, w: signed distance
        let expected_output: Vec<PodVector> = vec![
            PodVector {x:  0.0   , y:  -1.0   , z:  0.0, w:  0.0,},
            PodVector {x:  SQRT_2, y:  -SQRT_2, z:  0.0, w:  0.0,},
        ];
        
        let actual_output = execute_code::<PositionAndDirection, PodVector>(bytemuck::cast_slice(&test_input), function_execution.as_str(), execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);

    }
    
    #[test]
    fn test_sample_signed_distance() {
        let identity_box_class = NamedSdf::new(SdfBox::new(Vector::new(1.0, 1.0, 1.0)), make_dummy_sdf_name(), );

        let shader_code = generate_code_for(&identity_box_class);
        
        let template = ShaderFunction::new("PositionAndDirection", "RayMarchStep", "sample_signed_distance")
            .with_position_and_direction_type()
            .with_binding_group(DATA_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(shader_code);

        let function_execution = make_executable(&template, 
            create_argument_formatter!("{argument}.position.xyz, {argument}.direction.xyz"));

        let instance_transformation = Affine::from_nonuniform_scale(1.0, 2.0, 3.0);
        let sdf_instance = SdfInstance::new(instance_transformation, SdfClassIndex(0), Linkage::new(ObjectUid(0), MaterialIndex(0)));
        let mut buffer = GpuReadySerializationBuffer::new(1, SdfInstance::SERIALIZED_QUARTET_COUNT);
        sdf_instance.serialize_into(&mut buffer);
        
        let mut execution_config = {
            let mut ware = ExecutionConfig::new();
            ware
                .set_data_binding_group(DATA_BINDING_GROUP)
                .set_entry_point(ComputeRoutineEntryPoint::TestDefault)
                .add_dummy_binding_group(1, vec![])
                .add_binding_group(0, vec![], vec![
                    // the only value we need (in uniforms) is sdf instances count which is 1
                    BindGroupSlot::new(0, bytemuck::cast_slice(&vec![1_u32; 48])),
                ])
            ;
            ware
        };
        
        execution_config.add_binding_group(2, vec![], vec![
            BindGroupSlot::new(1, buffer.backend()),
        ]);
        
        let test_input = [
            PositionAndDirection {
                position:  PodVector::new(1.5, 0.0, 0.0),
                direction: PodVector::new(0.0, 0.0, 1.0),
            },
            PositionAndDirection {
                position:  PodVector::new(0.0, 0.0,  4.0),
                direction: PodVector::new(0.0, 0.0, -1.0),
            },
            PositionAndDirection {
                position:  PodVector::new(0.0, 0.5, 0.0),
                direction: PodVector::new(0.0, 1.0, 0.0),
            },
            PositionAndDirection {
                position:  PodVector::new(-1.0, -2.0, -3.0),
                direction: PodVector::new( 0.0,  1.0,  0.0),
            },
        ];

        // xyz: shifted position, w: signed distance
        let expected_output: Vec<PodVector> = vec![
            PodVector {x:  1.5, y:  0.0, z:  1.5, w:  1.5,},
            PodVector {x:  0.0, y:  0.0, z:  3.0, w:  1.0,},
            PodVector {x:  0.0, y: -1.0, z:  0.0, w: -1.5,},
            PodVector {x: -1.0, y: -2.0, z: -3.0, w:  0.0,},
        ];
        
        let actual_output = execute_code::<PositionAndDirection, PodVector>(bytemuck::cast_slice(&test_input), function_execution.as_str(), execution_config);
        
        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[must_use]
    fn generate_code_for(sdf: &NamedSdf) -> String {
        let mut registrator = SdfRegistrator::new();
        registrator.add(&sdf);
        
        let warehouse = SdfWarehouse::new(registrator);
        warehouse.sdf_classes_code().to_string()
    }

    #[must_use]
    fn make_dummy_sdf_name() -> UniqueSdfClassName {
        UniqueSdfClassName::new("some_box".to_string())
    }
}
