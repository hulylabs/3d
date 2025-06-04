#[cfg(test)]
mod tests {
    use std::f32::consts::SQRT_2;
    use crate::geometry::alias::Vector;
    use crate::geometry::transform::Affine;
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
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
    use crate::utils::tests::assert_utils::tests::assert_eq;
    use crate::tests::gpu_code_execution::tests::{execute_code, BindGroupSlot, ExecutionConfig};
    use crate::utils::object_uid::ObjectUid;
    use bytemuck::{Pod, Zeroable};
    use std::fmt::Write;
    use cgmath::Matrix4;
    use crate::gpu::pipelines_factory::ComputeRoutineEntryPoint;
    use crate::objects::material::Material;
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction, TypeDeclaration};
    use crate::utils::tests::common_values::tests::COMMON_GPU_EVALUATIONS_EPSILON;

    const TEST_DATA_IO_BINDING_GROUP: u32 = 3;
    
    const DUMMY_SDF_SELECTION_CODE: &str = "fn sdf_select(value: f32, vector: vec3f) -> f32 { return 0.0; }";

    #[repr(C)]
    #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
    struct TriangleAndRay { // ray x,y,z are packed into the a,b,c's 'w' coordinate
        a: PodVector,
        b: PodVector,
        c: PodVector,
        ray_origin: PodVector,
    }
    
    #[repr(C)]
    #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
    struct PositionAndDirection {
        position: PodVector,
        direction: PodVector,
    }
    
    #[repr(C)]
    #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
    struct ShadowInput {
        position: PodVector,
        to_light: PodVector,
        traverse_parameters: PodVector, // x - light size, y - min_ray_offset, z - max_ray_offset
    }
    
    impl ShaderFunction {
        #[must_use]
        fn with_position_and_direction_type(self: ShaderFunction) -> ShaderFunction {
            self.with_custom_type(
                TypeDeclaration::new("PositionAndDirection", "position", "vec4f")
                    .with_field("direction", "vec4f")
            )
        }
    }

    #[test]
    fn test_hit_triangle() {
        let template = ShaderFunction::new("TriangleAndRay", "vec4f", "hit_triangle_t")
            .with_custom_type(
                TypeDeclaration::new("TriangleAndRay", "a", "vec3f")
                    .with_field("ray_x", "f32")
                    .with_field("b", "vec3f")
                    .with_field("ray_y", "f32")
                    .with_field("c", "vec3f")
                    .with_field("ray_z", "f32")
                    .with_field("ray_origin", "vec3f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_SDF_SELECTION_CODE)
            .with_additional_shader_code(
                r#"fn hit_triangle_t(triangle: Triangle, ray: Ray) -> vec4f 
                { if (hit_triangle(triangle, 0.0, 1000.0, ray)) { return vec4f(hitRec.p, 1.0); } return vec4f(0.0); }"#
            );

        let function_execution = make_executable(&template,
        create_argument_formatter!(
            "Triangle({argument}.a, {argument}.b, {argument}.c, vec3f(0), vec3f(0), 13, vec3f(0), u32(3)), \
            Ray({argument}.ray_origin, vec3f({argument}.ray_x, {argument}.ray_y, {argument}.ray_z))")
        );

        let mut execution_config = config_empty_bindings();
        execution_config.add_binding_group(2, vec![], vec![
            // dummy material
            BindGroupSlot::new(3, bytemuck::cast_slice(&vec![0_u32; Material::SERIALIZED_QUARTET_COUNT * size_of::<u32>()])),
        ]);
        
        let test_input = [
            TriangleAndRay {
                a: PodVector::new_full(-2.0, 1.0, 0.0, -1.0), 
                b: PodVector::new_full(-2.0, 0.0, 1.0,  0.0), 
                c: PodVector::new_full(-2.0, 1.0, 1.0,  0.0),
                ray_origin: PodVector::new(3.0, 0.8, 0.8),
            },
            TriangleAndRay {
                a: PodVector::new_full(-2.0, 1.0, 0.0, -1.0),
                b: PodVector::new_full(-2.0, 0.0, 1.0,  0.0),
                c: PodVector::new_full(-2.0, 1.0, 1.0,  0.0),
                ray_origin: PodVector::new(1.0, 0.5, 0.5),
            },
            TriangleAndRay {
                a: PodVector::new_full(-2.0, 0.0, 0.0, -1.0),
                b: PodVector::new_full(-2.0, 0.0, 1.0,  0.0),
                c: PodVector::new_full(-2.0, 1.0, 0.0,  0.0),
                ray_origin: PodVector::new(2.0, 0.5, 0.5),
            },
        ];

        // xyz: shifted position, w: signed distance
        let expected_output: Vec<PodVector> = vec![
            PodVector {x: -2.0, y: 0.8, z: 0.8, w: 1.0,},
            PodVector {x: -2.0, y: 0.5, z: 0.5, w: 1.0,},
            PodVector {x: -2.0, y: 0.5, z: 0.5, w: 1.0,},
        ];

        let actual_output = execute_code::<TriangleAndRay, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test]
    fn test_shadow() {
        let identity_box_class = NamedSdf::new(SdfBox::new(Vector::new(1.0, 1.0, 1.0)), make_dummy_sdf_name(), );

        let shader_code = generate_code_for(&identity_box_class);

        let template = ShaderFunction::new("ShadowInput", "f32", "shadow")
            .with_custom_type(
                TypeDeclaration::new("ShadowInput", "position", "vec4f")
                    .with_field("to_light", "vec4f")
                    .with_field("light_size", "f32")
                    .with_field("min_ray_offset", "f32")
                    .with_field("max_ray_offset", "f32")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(shader_code);

        let function_execution = make_executable(&template,
        create_argument_formatter!("{argument}.position.xyz, {argument}.to_light.xyz, {argument}.light_size, {argument}.min_ray_offset, {argument}.max_ray_offset"));

        /*
        case 1:               case 2:
              * - light         * - light 
        |----------|            
        |          |            
        |          |            
        |__________|           
                                
             * - position       * - position 
        ________________________________________
        */
        
        let instance_transformation = Affine::from_nonuniform_scale(1.0, 3.0, 1.0);
        let buffer = make_single_serialized_sdf_instance(instance_transformation);
        let execution_config = config_sdf_tracing(buffer);

        let test_input = [
            ShadowInput {
                position:            PodVector::new(0.0, -1.0, 0.0),
                to_light:            PodVector::new(0.0,  1.0, 1.0),
                traverse_parameters: PodVector::new(1.0,  0.0, 8.0), 
            },
            ShadowInput {
                position:            PodVector::new(10.0, -1.0, 0.0),
                to_light:            PodVector::new( 0.0,  1.0, 1.0),
                traverse_parameters: PodVector::new( 1.0,  0.0, 3.0),
            },
        ];
        
        let expected_output: Vec<f32> = vec![
            0.0,
            1.0,
        ];

        let actual_output = execute_code::<ShadowInput, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test]
    fn test_perfect_reflection_evaluation() {
        /* The main objective here is to test the 'evaluate_reflection' function with the
        zero roughness passed to it.*/
        
        let template = ShaderFunction::new("PositionAndDirection", "vec4f", "evaluate_reflection_t")
            .with_position_and_direction_type()
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_SDF_SELECTION_CODE)
            .with_additional_shader_code(
            r#"fn evaluate_reflection_t(incident: vec3f, normal: vec3f) -> vec4f 
                { return vec4f(evaluate_reflection(incident, normal, vec3f(0.0), 0.0), 0.0); }"#
            );

        let function_execution = make_executable(&template, 
        create_argument_formatter!("{argument}.position.xyz, {argument}.direction.xyz"));

        let execution_config = config_empty_bindings();

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
        
        let actual_output = execute_code::<PositionAndDirection, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }
    
    #[test]
    fn test_sample_signed_distance() {
        let identity_box_class = NamedSdf::new(SdfBox::new(Vector::new(1.0, 1.0, 1.0)), make_dummy_sdf_name(), );

        let shader_code = generate_code_for(&identity_box_class);
        
        let template = ShaderFunction::new("PositionAndDirection", "f32", "sample_signed_distance")
            .with_position_and_direction_type()
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(shader_code);

        let function_execution = make_executable(&template, 
        create_argument_formatter!("{argument}.position.xyz, {argument}.direction.xyz"));

        let instance_transformation = Affine::from_nonuniform_scale(1.0, 2.0, 3.0);
        let buffer = make_single_serialized_sdf_instance(instance_transformation);
        let execution_config = config_sdf_tracing(buffer);
        
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
        let expected_output: Vec<f32> = vec![
             1.5,
             1.0,
            -1.5,
             0.0,
        ];
        
        let actual_output = execute_code::<PositionAndDirection, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);
        
        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    impl ExecutionConfig {
        #[must_use]
        fn common_test_config(&mut self) -> &mut Self {
            self
                .set_data_binding_group(TEST_DATA_IO_BINDING_GROUP)
                .set_entry_point(ComputeRoutineEntryPoint::TestDefault)
        }
    }
    
    #[must_use]
    fn config_empty_bindings() -> ExecutionConfig {
        let mut ware = ExecutionConfig::new();
        ware
            .common_test_config()
            .add_dummy_binding_group(0, vec![])
            .add_dummy_binding_group(1, vec![])
            .add_dummy_binding_group(2, vec![])
        ;
        ware
    }
    
    #[must_use]
    fn config_sdf_tracing(buffer: GpuReadySerializationBuffer) -> ExecutionConfig {
        let mut execution_config = {
            let mut ware = ExecutionConfig::new();
            ware
                .common_test_config()
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
        execution_config
    }
    
    #[must_use]
    fn make_single_serialized_sdf_instance(instance_transformation: Matrix4<f64>) -> GpuReadySerializationBuffer {
        let dummy_linkage = Linkage::new(ObjectUid(0), MaterialIndex(0));
        let sdf_instance = SdfInstance::new(instance_transformation, SdfClassIndex(0), dummy_linkage);
        let mut buffer = GpuReadySerializationBuffer::new(1, SdfInstance::SERIALIZED_QUARTET_COUNT);
        sdf_instance.serialize_into(&mut buffer);
        buffer
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
        UniqueSdfClassName::new("some_sdf".to_string())
    }
}
