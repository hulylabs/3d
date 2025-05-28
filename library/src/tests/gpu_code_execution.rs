#[cfg(test)]
pub(crate) mod tests {
    use std::collections::{HashMap, HashSet};
    use crate::gpu::compute_pipeline::ComputePipeline;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::output::duplex_layer::DuplexLayer;
    use crate::gpu::output::frame_buffer_layer::SupportUpdateFromCpu;
    use crate::gpu::resources::{ComputeRoutineEntryPoint, Resources};
    use bytemuck::{Pod, Zeroable};
    use wgpu::wgt::PollType;
    use wgpu::BufferUsages;

    struct BindGroup {
        index: u32,
        dummy_slots: HashSet<u32>,
        slots: Vec<BindGroupSlot>,
    }

    pub(crate) struct BindGroupSlot {
        index: u32,
        data: Vec<u8>,
    }

    impl BindGroupSlot {
        #[must_use]
        pub(crate) fn new(index: u32, data: &[u8]) -> Self {
            Self { index, data: data.to_vec() }
        }
    }

    pub(crate) struct ExecutionConfig {
        data_binding_group: u32,
        entry_point: ComputeRoutineEntryPoint,
        bind_groups: HashMap<u32, BindGroup>,
    }
    
    impl ExecutionConfig {
        pub(crate) fn new() -> Self {
            Self { 
                data_binding_group: 0, 
                entry_point: ComputeRoutineEntryPoint::Default,
                bind_groups: HashMap::new(),
            }
        }

        pub(crate) fn set_data_binding_group(&mut self, data_binding_group: u32) -> &mut Self {
            self.data_binding_group = data_binding_group;
            self
        }

        pub(crate) fn add_dummy_binding_group(&mut self, binding_group: u32, slots: Vec<u32>) -> &mut Self {
            assert_ne!(self.data_binding_group, binding_group, "can't stab test data binding group");
            self.bind_groups.insert(binding_group, BindGroup {
                index: binding_group, 
                dummy_slots: slots.into_iter().collect(), 
                slots: Vec::new()
            });
            self
        }

        pub(crate) fn add_binding_group(&mut self, binding_group: u32, slots_to_stab: Vec<u32>, slots: Vec<BindGroupSlot>) -> &mut Self {
            assert_ne!(self.data_binding_group, binding_group, "can't set test data binding group");
            self.bind_groups.insert(binding_group, BindGroup {
                index: binding_group, 
                dummy_slots: slots_to_stab.into_iter().collect(), 
                slots
            });
            self
        }

        pub(crate) fn set_entry_point(&mut self, entry_point: ComputeRoutineEntryPoint) -> &mut Self {
            self.entry_point = entry_point;
            self
        }
    }
    
    impl Default for ExecutionConfig {
        fn default() -> Self {
            Self::new()
        }
    }
    
    #[must_use]
    pub(crate) fn execute_code<TInput, TOutput>(input: &[TInput], gpu_code: &str, config: ExecutionConfig) -> Vec<TOutput> 
    where TInput: Zeroable + Pod, TOutput: Zeroable + Pod
    {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context.clone(), wgpu::TextureFormat::Rgba8Unorm);

        let module = resources.create_shader_module("test GPU function execution", gpu_code);

        let input_buffer = resources.create_storage_buffer_write_only("input", bytemuck::cast_slice(input));
        let buffer_size = FrameBufferSize::new(input.len() as u32, 1);
        let mut output_buffer = DuplexLayer::<TOutput>::new(context.device(), buffer_size, SupportUpdateFromCpu::Yes, "output");

        let mut pipeline = ComputePipeline::new(resources.create_compute_pipeline(config.entry_point, &module));
        pipeline.setup_bind_group(config.data_binding_group, Some("test data"), context.device(), |bind_group|{
            bind_group.add_entry(0, input_buffer.clone());
            bind_group.add_entry(1, output_buffer.gpu_copy());
        });

        let universal_usage = BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::UNIFORM;
        let dummy_buffer = resources.create_buffer("dummy", universal_usage, &vec![0_u8; 256]);
        for (_, group) in config.bind_groups {
            pipeline.setup_bind_group(group.index, None, context.device(), |bind_group|{
                for slot in group.dummy_slots {
                    bind_group.add_entry(slot, dummy_buffer.clone());
                }
                for slot in group.slots {
                    let buffer = resources.create_buffer("custom", universal_usage, &slot.data);
                    bind_group.add_entry(slot.index, buffer.clone());
                }
            });
        }

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