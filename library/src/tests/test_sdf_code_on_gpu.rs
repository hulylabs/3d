#[cfg(test)]
mod tests {
    use std::fmt::Write;
    use std::rc::Rc;
    use wgpu::wgt::PollType;
    use crate::geometry::alias::{Point, Vector};
    use crate::gpu::compute_pipeline::ComputePipeline;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::output::duplex_layer::DuplexLayer;
    use crate::gpu::output::frame_buffer_layer::SupportUpdateFromCpu;
    use crate::gpu::resources::{ComputeRoutine, Resources};
    use crate::scene::sdf::code_generator::{SdfCodeGenerator, SdfRegistrator};
    use crate::scene::sdf::named_sdf::{NamedSdf, UniqueName};
    use crate::scene::sdf::sdf::Sdf;
    use crate::scene::sdf::sdf_box::SdfBox;
    use crate::scene::sdf::sdf_sphere::SdfSphere;
    use crate::scene::sdf::sdf_union::SdfUnion;
    use crate::scene::sdf::shader_function_name::FunctionName;
    use crate::serialization::pod_vector::PodVector;

    #[test]
    fn test_spheres_union_sdf_execution() {
        let union = SdfUnion::new(
            SdfSphere::new_offset(2.0, Point::new(0.0,  7.0, 0.0)),
            SdfSphere::new_offset(2.0, Point::new(0.0, -7.0, 0.0)),
        );

        let input_points = [
            PodVector { x: 0.0, y: 0.0 , z: 0.0 , w: 0.0 },
            
            PodVector { x: 0.0 , y: 5.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 7.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 9.0 , z: 0.0 , w: 0.0 },

            PodVector { x: 0.0 , y: -5.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: -7.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: -9.0 , z: 0.0 , w: 0.0 },
        ];

        let expected_signed_distances: Vec<f32> = vec![
             5.0,
             
             0.0,
            -2.0,
             0.0,

             0.0, 
            -2.0,
             0.0,
        ];

        test_sdf_evaluation(union, "spheres_union", &input_points, &expected_signed_distances);
    }
    
    #[test]
    fn test_single_sphere_sdf_execution() {
        let sphere = SdfSphere::new(17.0);

        let input_points = [
            PodVector { x: 13.0, y: 0.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 17.0, z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 0.0 , z: 23.0, w: 0.0 },
        ];

        let expected_signed_distances: Vec<f32> = vec![
            -4.0, 0.0, 6.0
        ];
        
        test_sdf_evaluation(sphere, "sphere", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_single_box_sdf_execution() {
        let a_box = SdfBox::new(Vector::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  2.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z:  3.0 , w: 0.0 },

            PodVector { x: -1.0 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y: -2.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z: -3.0 , w: 0.0 },

            PodVector { x:  0.7 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  1.7 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z:  2.7 , w: 0.0 },

            PodVector { x:  1.1 , y:  2.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  2.2 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  2.0 , z:  5.0 , w: 0.0 },
        ];

        let expected_signed_distances: Vec<f32> = vec![
             0.0,  0.0,  0.0,
             0.0,  0.0,  0.0,
             
            -0.3, -0.3, -0.3,
             0.1,  0.2,  2.0,
        ];

        test_sdf_evaluation(a_box, "box", &input_points, &expected_signed_distances);
    }
    
    fn test_sdf_evaluation(sdf: Rc<dyn Sdf>, name: &str, sample_positions: &[PodVector], expected_distances: &Vec<f32>) {
        let named = NamedSdf::new(sdf, UniqueName(name.to_string()));

        let mut registrator = SdfRegistrator::new();
        registrator.add(&named);

        let generator = SdfCodeGenerator::new(registrator);

        let mut shader_code: String = String::new();
        let function_to_call = generator.generate_unique_code_for(&named, &mut shader_code);
        generator.generate_shared_code(&mut shader_code);

        let actual_distances = execute_function(&sample_positions, function_to_call, shader_code);

        let epsilon = 1e-7;
        assert_eq(&actual_distances, expected_distances, epsilon);
    }
    
    fn assert_eq(left: &[f32], right: &[f32], epsilon: f32) {
        assert_eq!(left.len(), right.len(), "ranges have different lengths");

        for (i, (x, y)) in left.iter().zip(right.iter()).enumerate() {
            assert!(
                (x - y).abs() < epsilon,
                "Values at index {} differ: {} vs {}",
                i, x, y
            );
        }
    }
    
    #[must_use]
    fn execute_function(input: &[PodVector], function_name: FunctionName, function_code: String) -> Vec<f32> {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context.clone(), wgpu::TextureFormat::Rgba8Unorm);

        let mut function_execution = make_function_execution(function_name);
        function_execution.write_str(function_code.as_str()).expect("shader code concatenation has failed");

        let module = resources.create_shader_module("test GPU function execution", function_execution.as_str());

        let input_buffer = resources.create_storage_buffer_write_only("input", bytemuck::cast_slice(input));
        let buffer_size = FrameBufferSize::new(input.len() as u32, 1);
        let mut output_buffer = DuplexLayer::<f32>::new(context.device(), buffer_size, SupportUpdateFromCpu::YES, "output");

        let mut pipeline = ComputePipeline::new(resources.create_compute_pipeline(ComputeRoutine::Default, &module));
        pipeline.setup_bind_group(0, None, context.device(), |bind_group|{
            bind_group.add_entry(0, input_buffer.clone());
            bind_group.add_entry(1, output_buffer.gpu_copy());
        });

        let mut encoder = context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pipeline.set_into_pass(&mut pass);
            let workgroup_count = input.len().div_ceil(64);
            pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }
        output_buffer.prepare_cpu_read(&mut encoder);
        context.queue().submit(Some(encoder.finish()));

        let copy_wait = output_buffer.read_cpu_copy();
        context.device().poll(PollType::Wait).expect("failed to poll the device");
        pollster::block_on(copy_wait);

        output_buffer.cpu_copy().clone()
    }

    const FUNCTION_EXECUTOR: &str = include_str!("point_function_executor.wgsl");

    #[must_use]
    fn make_function_execution(name: FunctionName) -> String {
        FUNCTION_EXECUTOR.to_string().replace("_FUNCTION_NAME_SLOT_TO_BE_FILLED_", name.0.as_str())
    }
}