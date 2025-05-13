#[cfg(test)]
pub(crate) mod tests {
    use crate::gpu::compute_pipeline::ComputePipeline;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::output::duplex_layer::DuplexLayer;
    use crate::gpu::output::frame_buffer_layer::SupportUpdateFromCpu;
    use crate::gpu::resources::{ComputeRoutine, Resources};
    use crate::serialization::pod_vector::PodVector;
    use wgpu::wgt::PollType;

    #[must_use]
    pub(crate) fn execute_code(input: &[PodVector], gpu_code: &str) -> Vec<f32> {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context.clone(), wgpu::TextureFormat::Rgba8Unorm);

        let module = resources.create_shader_module("test GPU function execution", gpu_code);

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
}