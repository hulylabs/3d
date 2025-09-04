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
    use crate::material::material_index::MaterialIndex;
    use crate::objects::common_properties::Linkage;
    use crate::objects::sdf_class_index::SdfClassIndex;
    use crate::objects::sdf_instance::SdfInstance;
    use crate::scene::camera::Camera;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::framework::sdf_registrator::SdfRegistrator;
    use crate::sdf::object::sdf_box::SdfBox;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::pod_vector::PodVector;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::tests::scaffolding::gpu_code_execution::tests::{create_checkerboard_texture_data, DataBindGroupSlot, ExecutionConfig, GpuCodeExecutionContext, SamplerBindGroupSlot, TextureBindGroupSlot};
    use crate::tests::scaffolding::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction, TypeDeclaration};
    use crate::utils::object_uid::ObjectUid;
    use crate::utils::tests::assert_utils::tests::assert_eq;
    use crate::utils::tests::common_values::tests::COMMON_GPU_EVALUATIONS_EPSILON;
    use bytemuck::{Pod, Zeroable};
    use cgmath::num_traits::float::FloatCore;
    use cgmath::{Array, ElementWise, EuclideanSpace, InnerSpace, Vector2};
    use std::f32::consts::SQRT_2;
    use std::time::Instant;
    use test_context::test_context;

    const TEST_DATA_IO_BINDING_GROUP: u32 = 3;
    
    const DUMMY_IMPLEMENTATIONS: &str = include_str!("dummy_implementations.wgsl");

    const DUMMY_TEXTURE_SELECTION: &str = "\
        fn procedural_texture_select(index: i32, position: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {\
        return vec3f(0.0);\
        }";

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

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_pixel_half_size(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("Differentials", "vec4f", "pixel_half_size_t")
            .with_custom_type(
                TypeDeclaration::new("Differentials", "ddx_and_ddy", "vec4f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
            .with_additional_shader_code(
                "fn pixel_half_size_t(request: Differentials) -> vec4f \
                { return vec4f(pixel_half_size(texture_atlas_page, request.ddx_and_ddy.xy, request.ddx_and_ddy.zw), 3.0, 7.0); }"
            );

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));
        let mut execution_config = config_empty_bindings();

        let texture_size = Vector2::<u32>::new(8, 16);
        let pixel_size = Vector2::<f32>::new(1.0 / texture_size.x as f32, 1.0 / texture_size.y as f32);
        let data = create_checkerboard_texture_data(texture_size.x, texture_size.y, 1);
        execution_config.set_texture_binding(0, TextureBindGroupSlot::new(2, texture_size, data), None);

        #[repr(C)] #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct Differentials {
            ddx_and_ddy: PodVector,
        }

        let test_input = [
            Differentials{ddx_and_ddy: PodVector::new_full(0.0, 0.0, 0.0, 0.0)},
            Differentials{ddx_and_ddy: PodVector::new_full(pixel_size.x, 0.0, 0.0, pixel_size.y)},
        ];

        let expected_pixel_half_size = pixel_size * 0.5;

        let expected_output = [
            PodVector {x: expected_pixel_half_size.x, y: expected_pixel_half_size.y, z: 3.0, w: 7.0,},
            PodVector {x: expected_pixel_half_size.x, y: expected_pixel_half_size.y, z: 3.0, w: 7.0,},
        ];

        let actual_output = fixture.get().execute_code::<Differentials, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq!(actual_output, expected_output);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_read_atlas_no_differentials(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("AtlasReadRequest", "vec4f", "read_atlas_t")
            .with_custom_type(
                TypeDeclaration::new("AtlasReadRequest", "local_space_position", "vec4f")
                    .with_field("atlas_region_mapping", "AtlasMapping")
                    .with_field("derivatives", "RayDerivatives")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
            .with_additional_shader_code(
                "fn read_atlas_t(request: AtlasReadRequest) -> vec4f \
                { return read_atlas(request.local_space_position.xyz, request.atlas_region_mapping, request.derivatives); }"
            );

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));
        let mut execution_config = config_empty_bindings();

        let texture_size = Vector2::<u32>::new(8, 4);
        let pixel_size = Vector2::<f32>::new(1.0 / texture_size.x as f32, 1.0 / texture_size.y as f32);
        let data = create_checkerboard_texture_data(texture_size.x, texture_size.y, 1);
        execution_config.set_texture_binding(0, TextureBindGroupSlot::new(2, texture_size, data), Some(SamplerBindGroupSlot::new(1)));

        #[repr(C)] #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct AtlasMapping {
            top_left_corner_and_size: PodVector,
            local_position_to_texture: [PodVector; 2],
            wrap_mode: [i32; 4],
        }

        #[repr(C)] #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct RayDerivatives {
            dp_dx: PodVector,
            dp_dy: PodVector,
        }

        #[repr(C)] #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct AtlasReadRequest {
            local_space_position: PodVector,
            atlas_region_mapping: AtlasMapping,
            differentials: RayDerivatives,
        }

        let zero_differentials = RayDerivatives { dp_dx: PodVector::new(0.0, 0.0, 0.0), dp_dy: PodVector::new(0.0, 0.0, 0.0), };

        let xu_yv_mapping = [
            PodVector::new_full(1.0, 0.0, 0.0, 0.0),
            PodVector::new_full(0.0, 1.0, 0.0, 0.0),
        ];

        let whole_texture_xu_yv = AtlasMapping {
            // whole texture
            top_left_corner_and_size: PodVector::new_full(0.0, 0.0, 1.0, 1.0),
            // x -> u, y -> v
            local_position_to_texture: xu_yv_mapping,
            wrap_mode: [0, 0, 0, 0],
        };
        let whole_texture_zu_xv = AtlasMapping {
            // whole texture
            top_left_corner_and_size: PodVector::new_full(0.0, 0.0, 1.0, 1.0),
            // z -> u, x -> v
            local_position_to_texture: [
                PodVector::new_full(0.0, 0.0, 1.0, 0.0),
                PodVector::new_full(1.0, 0.0, 0.0, 0.0),
            ],
            wrap_mode: [0, 0, 0, 0],
        };

        /*
        Our texture:

             A
             |
         W [B W] B W  B W  B
         B [W B] W B [W B] W
         W  B W  B W [B W] B
         B  W B  W B [W B] W
                       |
                       B
        */

        let a_region = PodVector::new_full(pixel_size.x * 1.0, pixel_size.y * 0.0, pixel_size.x * 2.0, pixel_size.y * 2.0);
        let b_region = PodVector::new_full(pixel_size.x * 5.0, pixel_size.y * 1.0, pixel_size.x * 2.0, pixel_size.y * 3.0);

        const TEXTURE_WRAP_MODE_REPEAT: i32 = 0;
        const TEXTURE_WRAP_MODE_CLAMP: i32 = 1;
        const TEXTURE_WRAP_MODE_DISCARD: i32 = 2;

        let test_input = [

            // whole texture mapping

            AtlasReadRequest{
                // center of top left pixel
                local_space_position: PodVector::new(pixel_size.x/2.0,pixel_size.y/2.0,0.0),
                atlas_region_mapping: whole_texture_xu_yv,
                differentials: zero_differentials
            },

            AtlasReadRequest{
                // center of bottom left pixel
                local_space_position: PodVector::new(1.0-pixel_size.y/2.0, 0.0,pixel_size.x/2.0),
                atlas_region_mapping: whole_texture_zu_xv,
                differentials: zero_differentials
            },

            AtlasReadRequest{
                // center of bottom right pixel
                local_space_position: PodVector::new(1.0-pixel_size.y/2.0, 0.0,1.0-pixel_size.x/2.0),
                atlas_region_mapping: whole_texture_zu_xv,
                differentials: zero_differentials
            },

            AtlasReadRequest{
                // center of top right pixel
                local_space_position: PodVector::new(1.0-pixel_size.x/2.0,pixel_size.y/2.0,0.0),
                atlas_region_mapping: whole_texture_xu_yv,
                differentials: zero_differentials
            },

            // regions mapping

            AtlasReadRequest{
                // right bottom corner of the 2x2 'A' region
                local_space_position: PodVector::new(0.25,0.75,0.0),
                atlas_region_mapping: AtlasMapping {
                    top_left_corner_and_size: a_region,
                    // x -> u + 0.5 , y -> v - 1 column shift
                    local_position_to_texture: [
                        PodVector::new_full(1.0, 0.0, 0.0, 0.5),
                        PodVector::new_full(0.0, 1.0, 0.0, 0.0),
                    ],
                    wrap_mode: [0, 0, 0, 0],
                },
                differentials: zero_differentials
            },

            AtlasReadRequest{
                //left top corner of the 2x3 'B' region
                local_space_position: PodVector::new(0.25,1.0 - (1.0/3.0)/2.0,0.0),
                atlas_region_mapping: AtlasMapping {
                    top_left_corner_and_size: b_region,
                    // x -> u, y -> v+2/3 - 2 rows shift
                    local_position_to_texture: [
                        PodVector::new_full(1.0, 0.0, 0.0, 0.0),
                        PodVector::new_full(0.0, 1.0, 0.0, -2.0/3.0),
                    ],
                    wrap_mode: [0, 0, 0, 0],
                },
                differentials: zero_differentials
            },

            // wrap_mode check - clamp

            AtlasReadRequest{
                local_space_position: PodVector::new(0.25, -(1.0/3.0)/2.0, 0.0),
                atlas_region_mapping: AtlasMapping {
                    top_left_corner_and_size: b_region,
                    local_position_to_texture: xu_yv_mapping,
                    wrap_mode: [TEXTURE_WRAP_MODE_REPEAT, TEXTURE_WRAP_MODE_CLAMP, 0, 0],
                },
                differentials: zero_differentials
            },

            AtlasReadRequest{
                local_space_position: PodVector::new(1.25, (1.0/3.0)/2.0, 0.0),
                atlas_region_mapping: AtlasMapping {
                    top_left_corner_and_size: b_region,
                    local_position_to_texture: xu_yv_mapping,
                    wrap_mode: [TEXTURE_WRAP_MODE_CLAMP, TEXTURE_WRAP_MODE_REPEAT, 0, 0],
                },
                differentials: zero_differentials
            },

            // wrap_mode check - discard

            AtlasReadRequest{
                local_space_position: PodVector::new(-0.25, (1.0/3.0)/2.0, 0.0),
                atlas_region_mapping: AtlasMapping {
                    top_left_corner_and_size: b_region,
                    local_position_to_texture: xu_yv_mapping,
                    wrap_mode: [TEXTURE_WRAP_MODE_DISCARD, TEXTURE_WRAP_MODE_CLAMP, 0, 0],
                },
                differentials: zero_differentials
            },

            AtlasReadRequest{
                local_space_position: PodVector::new(0.25, 1.0 + (1.0/3.0)/2.0, 0.0),
                atlas_region_mapping: AtlasMapping {
                    top_left_corner_and_size: b_region,
                    local_position_to_texture: xu_yv_mapping,
                    wrap_mode: [TEXTURE_WRAP_MODE_CLAMP, TEXTURE_WRAP_MODE_DISCARD, 0, 0],
                },
                differentials: zero_differentials
            },
        ];

        let expected_output = [
            // whole texture mapping
            PodVector {x: 1.0, y: 1.0, z: 1.0, w: 1.0,},
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 1.0,},
            PodVector {x: 1.0, y: 1.0, z: 1.0, w: 1.0,},
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 1.0,},
            // regions mapping
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 1.0,},
            PodVector {x: 1.0, y: 1.0, z: 1.0, w: 1.0,},
            // wrap_mode check - clamp
            PodVector {x: 1.0, y: 1.0, z: 1.0, w: 1.0,},
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 1.0,},
            // wrap check - discard
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 0.0,},
            PodVector {x: 0.0, y: 0.0, z: 0.0, w: 0.0,},
        ];

        let actual_output = fixture.get().execute_code::<AtlasReadRequest, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_inside_aabb(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("AabbAndPoint", "f32", "inside_aabb_t")
            .with_custom_type(
                TypeDeclaration::new("AabbAndPoint", "aabb_min", "vec3f")
                    .with_field("aabb_max", "vec3f")
                    .with_field("point", "vec3f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
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

        let actual_output = fixture.get().execute_code::<AabbAndPoint, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);

    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_ray_to_pixel(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("PixelSetup", "Ray", "ray_to_pixel")
            .with_custom_type(
                TypeDeclaration::new("PixelSetup", "fov_and_camera_origin", "vec4f")
                    .with_field("pixel", "Pixel")
                    .with_field("sub_pixel_x", "f32")
                    .with_field("sub_pixel_y", "f32")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS);

        let function_execution = make_executable(&template,
            create_argument_formatter!("Camera({argument}.fov_and_camera_origin.x, {argument}.fov_and_camera_origin.yzw), {argument}.pixel, {argument}.sub_pixel_x, {argument}.sub_pixel_y"));

        let camera = Camera::new_perspective_camera(3.0, Point::new(3.0, -7.0, 5.0));
        let frame_buffer_size = FrameBufferSize::new(2, 2);
        let uniforms = Uniforms::new(frame_buffer_size, camera, 3, Instant::now().elapsed());

        let mut execution_config = ExecutionConfig::new();
        execution_config
            .common_test_config()
            .set_storage_binding_group(0, vec![], vec![
                DataBindGroupSlot::new(0, uniforms.serialize().backend()),
            ])
            .set_dummy_binding_group(1, vec![], vec![], vec![])
            .set_dummy_binding_group(2, vec![], vec![], vec![])
            ;

        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct PixelSetup {
            fov_and_origin: PodVector,
            pixel_and_sub_pixel: PodVector,
        }

        // TODO: this duplicates a line from the tracer shader
        let fov_factor = 1.0 / (60.0 * (std::f32::consts::PI / 180.0) / 2.0).tan();
        let origin = Point::new(0.0, 0.0, 3.0);

        let test_input: Vec<PixelSetup> = vec![
            PixelSetup { fov_and_origin: PodVector::new_full(fov_factor, origin.x as f32, origin.y as f32, origin.z as f32),
                pixel_and_sub_pixel: PodVector::new_full(0.0, 0.0, 0.1, 0.9),
            },
            PixelSetup { fov_and_origin: PodVector::new_full(fov_factor, origin.x as f32, origin.y as f32, origin.z as f32),
                pixel_and_sub_pixel: PodVector::new_full(1.0, 0.0, 0.4, 0.6),
            },
            PixelSetup { fov_and_origin: PodVector::new_full(fov_factor, origin.x as f32, origin.y as f32, origin.z as f32),
                pixel_and_sub_pixel: PodVector::new_full(0.0, 1.0, 0.6, 0.4),
            },
            PixelSetup { fov_and_origin: PodVector::new_full(fov_factor, origin.x as f32, origin.y as f32, origin.z as f32),
                pixel_and_sub_pixel: PodVector::new_full(1.0, 1.0, 0.9, 0.1),
            },
        ];

        let pod_origin = PodVector::new_full(origin.x as f32, origin.y as f32, origin.z as f32, 0.0f32);
        let expected_output: Vec<PositionAndDirection> = vec![
            PositionAndDirection {
                position: pod_origin, direction: PodVector::new(0.6309147, -0.76439905, -0.13281836) ,
            },
            PositionAndDirection {
                position: pod_origin, direction: PodVector::new(0.40278503, -0.7445488, 0.53236383),
            },
            PositionAndDirection {
                position: pod_origin, direction: PodVector::new(0.32156247, -0.94559544, -0.04946544),
            },
            PositionAndDirection {
                position: pod_origin, direction: PodVector::new(0.044365983, -0.811256, 0.58300555),
            },
        ];

        let actual_output = fixture.get().execute_code::<PixelSetup, PositionAndDirection >(&test_input, function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_transform_ray_parameter(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("RayParameterTransformation", "f32", "transform_ray_parameter")
            .with_custom_type(
                TypeDeclaration::new("RayParameterTransformation", "matrix", "mat3x4f")
                    .with_field("ray_origin", "vec3f")
                    .with_field("ray_direction", "vec3f")
                    .with_field("parameter_and_transformed_origin", "vec4f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS);

        let function_execution = make_executable(&template,
            create_argument_formatter!("{argument}.matrix, Ray({argument}.ray_origin, {argument}.ray_direction), {argument}.parameter_and_transformed_origin.x, {argument}.parameter_and_transformed_origin.yzw"));

        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct RayParameterTransformation {
            matrix_columns: [PodVector; 3],
            ray_origin: PodVector,
            ray_direction: PodVector,
            parameter_and_transformed_origin: PodVector,
        }

        let common_matrix = [
            PodVector { x: 2.0, y: 0.0, z: 0.0, w: 2.0, },
            PodVector { x: 0.0, y: 1.0, z: 0.0, w: 4.0, },
            PodVector { x: 1.0, y: 0.0, z: 1.0, w: 6.0, },
        ];
        let test_input: Vec<RayParameterTransformation> = vec![
            RayParameterTransformation {
                matrix_columns: common_matrix,
                ray_origin: PodVector {x: -3.0, y: -7.0, z: -5.0, w: 1.0,},
                ray_direction: PodVector {x: 1.0, y: 1.0, z: 1.0, w: 0.0,},
                parameter_and_transformed_origin: PodVector {x: 4.0, y: -4.0, z: -3.0, w: 1.0,},
            },
            RayParameterTransformation {
                matrix_columns: common_matrix,
                ray_origin: PodVector {x: 0.0, y: 0.0, z: 0.0, w: 1.0,},
                ray_direction: PodVector {x: 1.0, y: 0.0, z: 0.0, w: 0.0,},
                parameter_and_transformed_origin: PodVector {x: 4.0, y: -4.0, z: -3.0, w: 1.0,},
            },
        ];

        let expected_output: Vec<f32> = vec![
            10.24695,
            18.05547,
        ];

        let execution_config = config_empty_bindings();
        let actual_output = fixture.get().execute_code::<RayParameterTransformation, f32>(&test_input, function_execution, execution_config);

        assert_eq!(actual_output, expected_output);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_signed_distance_normal(fixture: &mut GpuCodeExecutionContext) {
        let box_class = NamedSdf::new(SdfBox::new(Vector::new(2.0, 2.0, 2.0)), make_dummy_sdf_name(), );

        let shader_code = generate_code_for(&box_class);

        let template = ShaderFunction::new("vec3f", "vec3f", "sample_signed_distance_t")
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_TEXTURE_SELECTION)
            .with_additional_shader_code(shader_code)
            .with_additional_shader_code(
                r#"fn sample_signed_distance_t(position: vec3f) -> vec3f {
                    let sdf = sdf[0];
                    return signed_distance_normal(sdf, position, 0.0);
                }"#
            );

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));

        let instance_transformation = Affine::from_nonuniform_scale(1.0, 2.0, 3.0);
        let sdf_buffer = make_single_serialized_sdf_instance(&box_class, &instance_transformation);
        let mut execution_config = ExecutionConfig::new();
        execution_config
            .common_test_config()
            .set_dummy_binding_group(1, vec![], vec![], vec![])
            .set_dummy_binding_group(0, vec![], vec![], vec![])
            .set_storage_binding_group(2, vec![], vec![
                DataBindGroupSlot::new(1, sdf_buffer.instances.backend()),
            ]);

        let test_input = [
            PodVector::new(2.0, 0.0, 0.0),
            PodVector::new(2.1, 0.0, 0.0),
            PodVector::new(-2.0, 0.0, 0.0),
            PodVector::new(-2.1, 0.0, 0.0),

            PodVector::new(0.0, 4.0, 0.0),
            PodVector::new(0.0, 4.1, 0.0),
            PodVector::new(0.0, -4.0, 0.0),
            PodVector::new(0.0, -4.1, 0.0),

            PodVector::new(0.0, 0.0, 6.0),
            PodVector::new(0.0, 0.0, 6.1),
            PodVector::new(0.0, 0.0, -6.0),
            PodVector::new(0.0, 0.0, -6.1),
        ];

        // xyz: shifted position, w: signed distance
        let expected_output: Vec<PodVector> = vec![
            PodVector::new(1.0, 0.0, 0.0),
            PodVector::new(1.0, 0.0, 0.0),
            PodVector::new(-1.0, 0.0, 0.0),
            PodVector::new(-1.0, 0.0, 0.0),

            PodVector::new(0.0, 1.0, 0.0),
            PodVector::new(0.0, 1.0, 0.0),
            PodVector::new(0.0, -1.0, 0.0),
            PodVector::new(0.0, -1.0, 0.0),

            PodVector::new(0.0, 0.0, 1.0),
            PodVector::new(0.0, 0.0, 1.0),
            PodVector::new(0.0, 0.0, -1.0),
            PodVector::new(0.0, 0.0, -1.0),
        ];

        let actual_output = fixture.get().execute_code::<PodVector, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), 1e-3);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_transform_point(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("MatrixAndPoint", "vec3f", "transform_point")
            .with_custom_type(
                TypeDeclaration::new("MatrixAndPoint", "matrix", "mat3x4f")
                    .with_field("point", "vec3f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS);

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}.matrix, {argument}.point"));

        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct MatrixAndPoint {
            matrix_columns: [PodVector; 3],
            point: PodVector,
        }

        let test_input: Vec<MatrixAndPoint> = vec![
            MatrixAndPoint {
                matrix_columns: [
                    PodVector {x: 0.0, y: 1.0, z: 0.0, w:  3.0,},
                    PodVector {x: 0.0, y: 0.0, z: 1.0, w:  7.0,},
                    PodVector {x: 1.0, y: 0.0, z: 0.0, w:  5.0,},
                ],
                point: PodVector {x: 1.0, y: 2.0, z:  3.0, w:  1.0,},
            },
            MatrixAndPoint {
                matrix_columns: [
                    PodVector {x: 0.0, y: 1.0, z: 1.0, w:  7.0,},
                    PodVector {x: 1.0, y: 0.0, z: 1.0, w:  3.0,},
                    PodVector {x: 1.0, y: 1.0, z: 0.0, w:  5.0,},
                ],
                point: PodVector {x: 1.0, y: 2.0, z:  3.0, w:  1.0,},
            },
        ];

        let execution_config = config_empty_bindings();
        let actual_output = fixture.get().execute_code::<MatrixAndPoint, PodVector>(&test_input, function_execution, execution_config);

        let expected_output: Vec<PodVector> = vec![
            PodVector {x: 5.0, y: 10.0, z: 6.0, w: 0.0,},
            PodVector {x: 12.0, y: 7.0, z: 8.0, w: 0.0,},
        ];

        assert_eq!(actual_output, expected_output);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_transform_transposed_vector(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("MatrixAndVector", "vec3f", "transform_transposed_vector")
            .with_custom_type(
                TypeDeclaration::new("MatrixAndVector", "matrix", "mat3x3f")
                    .with_field("vector", "vec3f")
            )
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS);

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}.matrix, {argument}.vector"));

        #[repr(C)]
        #[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
        struct MatrixAndVector {
            matrix_columns: [PodVector; 3],
            vector: PodVector,
        }

        let test_input: Vec<MatrixAndVector> = vec![
            MatrixAndVector {
                matrix_columns: [
                    PodVector {x: 0.0, y: 0.0, z: 1.0, w:  3.0,},
                    PodVector {x: 0.0, y: 1.0, z: 0.0, w:  7.0,},
                    PodVector {x: 1.0, y: 0.0, z: 0.0, w:  5.0,},
                ],
                vector: PodVector {x: 1.0, y: 2.0, z:  3.0, w:  1.0,},
            },
            MatrixAndVector {
                matrix_columns: [
                    PodVector {x: 0.0, y: 1.0, z: 0.0, w:  7.0,},
                    PodVector {x: 1.0, y: 0.0, z: 0.0, w:  3.0,},
                    PodVector {x: 0.0, y: 0.0, z: 1.0, w:  5.0,},
                ],
                vector: PodVector {x: 1.0, y: 2.0, z:  3.0, w:  1.0,},
            },
            MatrixAndVector {
                matrix_columns: [
                    PodVector {x: 0.0, y: 1.0, z: 1.0, w:  7.0,},
                    PodVector {x: 1.0, y: 0.0, z: 1.0, w:  3.0,},
                    PodVector {x: 0.0, y: 0.0, z: 1.0, w:  5.0,},
                ],
                vector: PodVector {x: 1.0, y: 2.0, z:  3.0, w:  1.0,},
            },
        ];

        let execution_config = config_empty_bindings();
        let actual_output = fixture.get().execute_code::<MatrixAndVector, PodVector>(&test_input, function_execution, execution_config);

        let expected_output: Vec<PodVector> = vec![
            PodVector {x: 3.0, y: 2.0, z: 1.0, w: 0.0,},
            PodVector {x: 2.0, y: 1.0, z: 3.0, w: 0.0,},
            PodVector {x: 2.0, y: 1.0, z: 6.0, w: 0.0,},
        ];

        assert_eq!(actual_output, expected_output);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_to_mat3x3(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("mat3x4f", "mat3x3f", "to_mat3x3")
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS);

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}"));

        let test_input: Vec<PodVector> = vec![
            PodVector {x: 1.0, y: 2.0,  z:   3.0, w:  4.0,},
            PodVector {x: 5.0, y: 6.0,  z:   7.0, w:  8.0,},
            PodVector {x: 9.0, y: 10.0, z:  11.0, w: 12.0,},
        ];

        let expected_output: Vec<PodVector> = vec![
            PodVector {x: 1.0, y:  2.0, z:  3.0, w: 0.0,},
            PodVector {x: 5.0, y:  6.0, z:  7.0, w: 0.0,},
            PodVector {x: 9.0, y: 10.0, z: 11.0, w: 0.0,},
        ];

        let execution_config = config_empty_bindings();
        let actual_output = fixture.get().execute_code::<PodVector, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq!(actual_output, expected_output);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_random_double(fixture: &mut GpuCodeExecutionContext) {
        let template = ShaderFunction::new("vec4f", "f32", "random_double_t")
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
            .with_additional_shader_code(
                r#"fn random_double_t(min: f32, max: f32, iterations_count: f32, take_min: f32) -> f32 {
                    var result = random_double(min, max);
                    for (var i = 0.0; i < iterations_count; i = i + 1.0) {
                        if (take_min == 0.0) {
                            result = min(result, random_double(min, max));
                        } else {
                            result = max(result, random_double(min, max));
                        }
                    }
                    return result;
                }"#
            );

        let function_execution = make_executable(&template, create_argument_formatter!("{argument}.x, {argument}.y, {argument}.z, {argument}.w"));

        let test_input: Vec<PodVector> = vec![
            PodVector {x: 0.0, y: 1.0,      z: 100.0 /*iterations count*/, w: 0.0 /* - take min*/,},
            PodVector {x: 0.0, y: 1.0,      z: 100.0 /*iterations count*/, w: 1.0 /* - take max*/,},

            PodVector {x: 0.0, y: 100.0,    z: 100.0 /*iterations count*/, w: 0.0 /* - take min*/,},
            PodVector {x: 0.0, y: 100.0,    z: 100.0 /*iterations count*/, w: 1.0 /* - take max*/,},

            PodVector {x: -113.0, y: 117.0, z: 100.0 /*iterations count*/, w: 0.0 /* - take min*/,},
            PodVector {x: -113.0, y: 117.0, z: 100.0 /*iterations count*/, w: 1.0 /* - take max*/,},

            PodVector {x: 3.7, y: 5.1,      z: 100.0 /*iterations count*/, w: 0.0 /* - take min*/,},
            PodVector {x: 3.7, y: 5.1,      z: 100.0 /*iterations count*/, w: 1.0 /* - take max*/,},

            PodVector {x: -3.7, y: 5.1,     z: 100.0 /*iterations count*/, w: 0.0 /* - take min*/,},
            PodVector {x: -3.7, y: 5.1,     z: 100.0 /*iterations count*/, w: 1.0 /* - take max*/,},

            PodVector {x: -300.7, y: -50.1, z: 100.0 /*iterations count*/, w: 0.0 /* - take min*/,},
            PodVector {x: -300.7, y: -50.1, z: 100.0 /*iterations count*/, w: 1.0 /* - take max*/,},
        ];

        let execution_config = config_empty_bindings();
        let actual_output = fixture.get().execute_code::<PodVector, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        for i in 0..test_input.len() {
            assert!(actual_output[i] >= test_input[i].x && actual_output[i] < test_input[i].y, "random_double failed for input: {:?}", test_input[i]);
        }
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_hit_aabb(fixture: &mut GpuCodeExecutionContext) {
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
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
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
        let actual_output = fixture.get().execute_code::<AabbAndRay, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_hit_triangle(fixture: &mut GpuCodeExecutionContext) {
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
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
            .with_additional_shader_code(
                r#"fn hit_triangle_t(triangle: Triangle, ray: Ray) -> vec4f 
                { if (hit_triangle(triangle, 0.0, 1000.0, ray)) { return vec4f(hitRec.global.position, 1.0); } return vec4f(0.0); }"#
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

        let actual_output = fixture.get().execute_code::<TriangleAndRay, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_shadow(fixture: &mut GpuCodeExecutionContext) {
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
            .with_additional_shader_code(DUMMY_TEXTURE_SELECTION)
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

        let actual_output = fixture.get().execute_code::<ShadowInput, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_perfect_reflection_evaluation(fixture: &mut GpuCodeExecutionContext) {
        /* The main goal here is to test the 'evaluate_reflection' function with the
        zero roughness passed to it.*/
        
        let template = ShaderFunction::new("PositionAndDirection", "vec4f", "evaluate_reflection_t")
            .with_position_and_direction_type()
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
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
        
        let actual_output = fixture.get().execute_code::<PositionAndDirection, PodVector>(bytemuck::cast_slice(&test_input), function_execution, execution_config);

        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    #[test_context(GpuCodeExecutionContext)]
    #[test]
    fn test_sample_signed_distance(fixture: &mut GpuCodeExecutionContext) {
        let identity_box_class = NamedSdf::new(SdfBox::new(Vector::new(1.0, 1.0, 1.0)), make_dummy_sdf_name(), );

        let shader_code = generate_code_for(&identity_box_class);
        
        let template = ShaderFunction::new("PositionAndDirection", "f32", "sample_signed_distance")
            .with_position_and_direction_type()
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_TEXTURE_SELECTION)
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
        
        let actual_output = fixture.get().execute_code::<PositionAndDirection, f32>(bytemuck::cast_slice(&test_input), function_execution, execution_config);
        
        assert_eq(bytemuck::cast_slice(&actual_output), bytemuck::cast_slice(&expected_output), COMMON_GPU_EVALUATIONS_EPSILON);
    }

    impl ExecutionConfig {
        fn common_test_config(&mut self) -> &mut Self {
            self
                .set_test_data_binding_group(TEST_DATA_IO_BINDING_GROUP)
                .set_entry_point(ComputeRoutineEntryPoint::TestDefault)
        }
    }
    
    #[must_use]
    fn config_empty_bindings() -> ExecutionConfig {
        let mut ware = ExecutionConfig::new();
        ware
            .common_test_config()
            .set_dummy_binding_group(0, vec![], vec![], vec![])
            .set_dummy_binding_group(1, vec![], vec![], vec![])
            .set_dummy_binding_group(2, vec![], vec![], vec![])
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
            .set_dummy_binding_group(1, vec![], vec![], vec![])
            .set_storage_binding_group(0, vec![], vec![
                DataBindGroupSlot::new(0, uniforms.serialize().backend()),
            ])
        ;
        ware
    }

    #[must_use]
    fn make_test_uniforms() -> Uniforms {
        let dummy_camera = Camera::new_orthographic_camera(1.0, Point::origin());
        let dummy_frame_buffer_size = FrameBufferSize::new(1, 1);
        Uniforms::new(dummy_frame_buffer_size, dummy_camera, 1, Instant::now().elapsed())
    }

    #[must_use]
    fn config_sdf_sampling(serialized_sdf: SdfInstances) -> ExecutionConfig {
        let mut execution_config = config_common_sdf_buffers();
        execution_config.set_storage_binding_group(2, vec![], vec![
            DataBindGroupSlot::new(1, serialized_sdf.instances.backend()),
            DataBindGroupSlot::new(5, serialized_sdf.inflated_bvh.backend()),
            DataBindGroupSlot::new(6, bytemuck::bytes_of(&[0_f32; 1])),
        ]);
        execution_config
    }

    #[must_use]
    fn config_sdf_shadow_sampling(serialized_sdf: SdfInstances) -> ExecutionConfig {
        let mut execution_config = config_common_sdf_buffers();
        let dummy_buffer = [0_u8; 96];
        execution_config.set_storage_binding_group(2, vec![], vec![
            DataBindGroupSlot::new(0, &dummy_buffer),
            DataBindGroupSlot::new(1, serialized_sdf.instances.backend()),
            DataBindGroupSlot::new(2, &dummy_buffer),
            DataBindGroupSlot::new(3, &dummy_buffer),
            DataBindGroupSlot::new(4, serialized_sdf.bvh.backend()),
            DataBindGroupSlot::new(5, serialized_sdf.inflated_bvh.backend()),
            DataBindGroupSlot::new(6, bytemuck::bytes_of(&[0_f32; 1])),
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
