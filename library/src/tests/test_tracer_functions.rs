#[cfg(test)]
mod tests {
    use crate::bvh::builder::build_serialized_bvh;
    use crate::container::bvh_proxies::proxy_of_sdf;
    use crate::container::sdf_warehouse::SdfWarehouse;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::transform::Affine;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::pipelines_factory::ComputeRoutineEntryPoint;
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
    use crate::gpu::uniforms::Uniforms;
    use crate::objects::common_properties::Linkage;
    use crate::objects::sdf_instance::SdfInstance;
    use crate::objects::sdf_class_index::SdfClassIndex;
    use crate::scene::camera::Camera;
    use crate::sdf::framework::code_generator::SdfRegistrator;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::object::sdf_box::SdfBox;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::pod_vector::PodVector;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::tests::gpu_code_execution::tests::{execute_code, BindGroupSlot, ExecutionConfig};
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction, TypeDeclaration};
    use crate::utils::object_uid::ObjectUid;
    use crate::utils::tests::assert_utils::tests::assert_eq;
    use crate::utils::tests::common_values::tests::COMMON_GPU_EVALUATIONS_EPSILON;
    use bytemuck::{Pod, Zeroable};
    use cgmath::num_traits::float::FloatCore;
    use cgmath::{Array, ElementWise, EuclideanSpace, InnerSpace};
    use std::f32::consts::SQRT_2;
    use std::fmt::Write;
    use crate::material::material_index::MaterialIndex;

    const TEST_DATA_IO_BINDING_GROUP: u32 = 3;
    
    const DUMMY_SDF_SELECTION_CODE: &str = "fn sdf_select(value: f32, vector: vec3f, time: f32) -> f32 { return 0.0; }";
    
    #[repr(C)]
    #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
    struct PositionAndDirection {
        position: PodVector,
        direction: PodVector,
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
    fn test_inside_aabb() {
        let template = ShaderFunction::new("AabbAndPoint", "f32", "inside_aabb_t")
            .with_custom_type(
                TypeDeclaration::new("AabbAndPoint", "aabb_min", "vec3f")
                    .with_field("aabb_max", "vec3f")
                    .with_field("point", "vec3f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_SDF_SELECTION_CODE)
            .with_additional_shader_code(
                "fn inside_aabb_t(data: AabbAndPoint) -> f32 \
                { if (inside_aabb(data.aabb_min, data.aabb_max, data.point)) { return 1.0; } else { return 0.0; } }"
            );

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));
        let execution_config = config_empty_bindings();

        #[repr(C)] #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct AabbAndPoint {
            aabb_min: PodVector,
            aabb_max: PodVector,
            point: PodVector,
        }

        let aabb_min = PodVector::new(-1.0, -2.0, -3.0, );
        let aabb_max = PodVector::new(4.0, 5.0, 6.0, );

        let test_input = [
            AabbAndPoint { aabb_min, aabb_max, point: aabb_min, },
            AabbAndPoint { aabb_min, aabb_max, point: aabb_max, },

            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  0.0,  0.0, ), },

            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new(-1.0,  0.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0, -2.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  0.0, -3.0, ), },

            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 4.0,  0.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  5.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  0.0,  6.0, ), },

            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 5.0,  0.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  6.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  0.0,  7.0, ), },

            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new(-2.0,  0.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0, -3.0,  0.0, ), },
            AabbAndPoint { aabb_min, aabb_max, point: PodVector::new( 0.0,  0.0, -4.0, ), },
        ];

        // xyz: shifted position, w: signed distance
        let expected_output: Vec<f32> = vec![
            1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        let actual_output = execute_code::<AabbAndPoint, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);

    }

    #[test]
    fn test_hit_aabb() {
        let template = ShaderFunction::new("AabbAndRay", "vec4f", "hit_aabb_t")
            .with_custom_type(
                TypeDeclaration::new("AabbAndRay", "box_min", "vec3f")
                    .with_field("tmin", "f32")
                    .with_field("box_max", "vec3f")
                    .with_field("tmax", "f32")
                    .with_field("ray_origin", "vec3f")
                    .with_field("ray_direction_inverted", "vec3f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_SDF_SELECTION_CODE)
            .with_additional_shader_code(
                r#"fn hit_aabb_t(data: AabbAndRay) -> vec4f 
                { let result = hit_aabb(data.box_min, data.box_max, data.tmin, data.tmax, data.ray_origin, data.ray_direction_inverted);
                  if (result.hit) { return vec4f(1.0, result.ray_parameter, 0.0, 0.0); } else { return vec4f(0.0, 0.0, 0.0, 0.0); } }"#
            );

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));

        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct AabbAndRay {
            box_and_ray_min: PodVector,
            box_and_ray_max: PodVector,
            ray_origin: PodVector,
            ray_direction_inverted: PodVector,
        }

        let box_and_ray_min = PodVector::new_full(0.0, 0.0, 0.0, 0.0);
        let box_and_ray_max = PodVector::new_full(1.0, 1.0, 1.0,  1.0);
        let test_input = [
            AabbAndRay {
                box_and_ray_min, box_and_ray_max,
                ray_origin: PodVector::new(-2.0, 0.5, 0.5),
                ray_direction_inverted: PodVector::new(1.0, f32::infinity(), f32::infinity()),
            },
            AabbAndRay {
                box_and_ray_min, box_and_ray_max,
                ray_origin: PodVector::new(-0.5, 0.5, 0.5),
                ray_direction_inverted: PodVector::new(1.0, f32::infinity(), f32::infinity()),
            },
            AabbAndRay {
                box_and_ray_min, box_and_ray_max,
                ray_origin: PodVector::new(1.5, 1.5, 1.5),
                ray_direction_inverted: to_pod(Vector::from_value(1.0).div_element_wise(Vector::from_value(-1.0).normalize())),
            },
        ];

        let expected_output: Vec<PodVector> = vec![
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 0.0,},
            PodVector {x: 1.0, y: 0.5, z: 0.0, w: 0.0,},
            PodVector {x: 1.0, y: Vector::from_value(0.5).magnitude() as f32, z: 0.0, w: 0.0,},
        ];

        let execution_config = config_empty_bindings();
        let actual_output = execute_code::<AabbAndRay, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
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

        let execution_config = config_empty_bindings();
        
        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct TriangleAndRay { // ray x,y,z are packed into the a,b,c's 'w' coordinate
            a: PodVector,
            b: PodVector,
            c: PodVector,
            ray_origin: PodVector,
        }
        
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

        let template = ShaderFunction::new("ShadowInput", "f32", "evaluate_soft_shadow")
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
        case 1:                  case 2:
                                            
              * - light            * - light 
        |----------|               
        |          |               
        |          |               
        |__________|              
                                   
             * - position          * - position 
        ________________________________________
        */
        
        let instance_transformation = Affine::from_nonuniform_scale(1.0, 3.0, 1.0);
        let buffer = make_single_serialized_sdf_instance(&identity_box_class, &instance_transformation);
        let execution_config = config_sdf_shadow_sampling(buffer);

        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct ShadowInput {
            position: PodVector,
            to_light: PodVector,
            traverse_parameters: PodVector, // x - light size, y - min_ray_offset, z - max_ray_offset
        }
        
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
        let buffer = make_single_serialized_sdf_instance(&identity_box_class, &instance_transformation);
        let execution_config = config_sdf_sampling(buffer);
        
        let test_input = [
            PositionAndDirection {
                position:  PodVector::new(1.5, 0.0, 0.0),
                direction: PodVector::new(0.0, 0.0, 1.0),
            },
            PositionAndDirection {
                position:  PodVector::new(1.1, 0.0, 0.0),
                direction: PodVector::new(0.0, 0.0, 1.0),
            },
            PositionAndDirection {
                position:  PodVector::new(0.0, 0.0,  3.1),
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
             1e9,
             0.3,
             0.1,
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
    fn config_common_sdf_buffers() -> ExecutionConfig {
        let mut uniforms = make_test_uniforms();
        uniforms.set_bvh_length(1);
        uniforms.set_parallelograms_count(0);
        
        let mut ware = ExecutionConfig::new();
        ware
            .common_test_config()
            .add_dummy_binding_group(1, vec![])
            .add_binding_group(0, vec![], vec![
                // the only value we need (in uniforms) is sdf instances count which is 1
                BindGroupSlot::new(0, uniforms.serialize().backend()),
            ])
        ;
        ware
    }

    #[must_use]
    fn make_test_uniforms() -> Uniforms {
        let dummy_camera = Camera::new_orthographic_camera(1.0, Point::origin());
        let dummy_frame_buffer_size = FrameBufferSize::new(1, 1);
        Uniforms::new(dummy_frame_buffer_size, dummy_camera, 1)
    }

    #[must_use]
    fn config_sdf_sampling(serialized_sdf: SdfInstances) -> ExecutionConfig {
        let mut execution_config = config_common_sdf_buffers();
        execution_config.add_binding_group(2, vec![], vec![
            BindGroupSlot::new(1, serialized_sdf.instances.backend()),
            BindGroupSlot::new(5, serialized_sdf.inflated_bvh.backend()),
            BindGroupSlot::new(6, bytemuck::bytes_of(&[0_f32; 1])),
        ]);
        execution_config
    }

    #[must_use]
    fn config_sdf_shadow_sampling(serialized_sdf: SdfInstances) -> ExecutionConfig {
        let mut execution_config = config_common_sdf_buffers();
        let dummy_buffer = [0_u8; 96];
        execution_config.add_binding_group(2, vec![], vec![
            BindGroupSlot::new(0, &dummy_buffer),
            BindGroupSlot::new(1, serialized_sdf.instances.backend()),
            BindGroupSlot::new(2, &dummy_buffer),
            BindGroupSlot::new(3, &dummy_buffer),
            BindGroupSlot::new(4, serialized_sdf.bvh.backend()),
            BindGroupSlot::new(5, serialized_sdf.inflated_bvh.backend()),
            BindGroupSlot::new(6, bytemuck::bytes_of(&[0_f32; 1])),
        ]);
        execution_config
    }
    
    struct SdfInstances {
        instances: GpuReadySerializationBuffer,
        bvh: GpuReadySerializationBuffer,
        inflated_bvh: GpuReadySerializationBuffer,
    }
    
    #[must_use]
    fn make_single_serialized_sdf_instance(class: &NamedSdf, instance_transformation: &Affine) -> SdfInstances {
        let dummy_linkage = Linkage::new(ObjectUid(0), MaterialIndex(0));
        
        let sdf_instance = SdfInstance::new(instance_transformation.clone(), 1.0, SdfClassIndex(0), dummy_linkage);
        let mut instances = GpuReadySerializationBuffer::new(1, SdfInstance::SERIALIZED_QUARTET_COUNT);
        sdf_instance.serialize_into(&mut instances);

        #[must_use]
        fn make_bvh(sdf: &NamedSdf, instance_transformation: &Affine, inflation: f64) -> GpuReadySerializationBuffer {
            let aabb = sdf.sdf().aabb().transform(&instance_transformation).extent_relative_inflate(inflation);
            let mut support = [proxy_of_sdf(0, aabb)];
            build_serialized_bvh(&mut support) 
        }

        let inflated_bvh = make_bvh(class, instance_transformation, 0.1);
        let bvh = make_bvh(class, instance_transformation, 0.0);
        
        SdfInstances { instances, bvh, inflated_bvh }
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
    
    #[must_use]
    fn to_pod(from: Vector) -> PodVector {
        PodVector::new(from.x as f32, from.y as f32, from.z as f32)
    }
}
