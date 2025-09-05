#[cfg(test)]
pub mod tests {
    use crate::gpu::compute_pipeline::ComputePipeline;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::headless_device::tests::create_headless_wgpu_vulkan_context;
    use crate::gpu::output::duplex_layer::DuplexLayer;
    use crate::gpu::output::frame_buffer_layer::SupportUpdateFromCpu;
    use crate::gpu::pipeline_code::PipelineCode;
    use crate::gpu::pipelines_factory::{ComputeRoutineEntryPoint, PipelinesFactory};
    use crate::gpu::resources::Resources;
    use crate::utils::tests::common_values::tests::COMMON_PRESENTATION_FORMAT;
    use bytemuck::{Pod, Zeroable};
    use cgmath::Vector2;
    use more_asserts::assert_gt;
    use std::collections::{HashMap, HashSet};
    use std::rc::Rc;
    use std::thread;
    use test_context::TestContext;
    use wgpu::BufferUsages;
    use crate::gpu::context::Context;
    use crate::utils::bitmap_utils::{BitmapSize, BYTES_IN_RGBA_QUARTET};

    pub(crate) struct DataBindGroupSlot {
        index: u32,
        data: Vec<u8>,
    }

    pub(crate) struct SamplerBindGroupSlot {
        index: u32,
    }

    pub(crate) struct TextureBindGroupSlot {
        index: u32,
        size: Vector2<u32>,
        data: Vec<u8>,
    }

    struct SlotClass<T> {
        dummy_slots: HashSet<u32>,
        slots: Vec<T>,
    }

    struct BindGroup {
        index: u32,
        storage_slots: SlotClass<DataBindGroupSlot>,
        sampler_slots: SlotClass<SamplerBindGroupSlot>,
        texture_slots: SlotClass<TextureBindGroupSlot>,
    }

    impl DataBindGroupSlot {
        #[must_use]
        pub fn new(index: u32, data: &[u8]) -> Self {
            Self { index, data: data.to_vec() }
        }
    }
    
    impl SamplerBindGroupSlot {
        pub fn new(index: u32) -> Self {
            Self { index }
        }
    }
    
    impl TextureBindGroupSlot {
        #[must_use]
        pub fn new(index: u32, size: Vector2<u32>, data: Vec<u8>) -> Self {
            assert_eq!((size.x * size.y) as usize * BYTES_IN_RGBA_QUARTET, data.len(), "data size mismatch");
            Self { index, size, data }
        }
    }

    pub(crate) struct ExecutionConfig {
        test_data_binding_group: u32,
        entry_point: ComputeRoutineEntryPoint,
        bind_groups: HashMap<u32, BindGroup>,
    }
    
    impl ExecutionConfig {
        pub(crate) fn new() -> Self {
            Self { 
                test_data_binding_group: 0,
                entry_point: ComputeRoutineEntryPoint::Default,
                bind_groups: HashMap::new(),
            }
        }

        pub(crate) fn set_test_data_binding_group(&mut self, data_binding_group: u32) -> &mut Self {
            self.test_data_binding_group = data_binding_group;
            self
        }

        pub(crate) fn set_dummy_binding_group(&mut self, binding_group: u32, storage_slots: Vec<u32>, sampler_slots: Vec<u32>, texture_slots: Vec<u32>, ) -> &mut Self {
            assert_ne!(self.test_data_binding_group, binding_group, "can't stab test data binding group");
            let dummy_bind_group = Self::make_dummy_bind_group(binding_group, storage_slots, sampler_slots, texture_slots);
            self.bind_groups.insert(binding_group, dummy_bind_group);
            self
        }

        fn make_dummy_bind_group(binding_group: u32, storage_slots: Vec<u32>, sampler_slots: Vec<u32>, texture_slots: Vec<u32>) -> BindGroup {
            BindGroup {
                index: binding_group,
                storage_slots: SlotClass::<DataBindGroupSlot> {
                    dummy_slots: storage_slots.into_iter().collect(),
                    slots: Vec::new(),
                },
                sampler_slots: SlotClass::<SamplerBindGroupSlot> {
                    dummy_slots: sampler_slots.into_iter().collect(),
                    slots: Vec::new(),
                },
                texture_slots: SlotClass::<TextureBindGroupSlot> {
                    dummy_slots: texture_slots.into_iter().collect(),
                    slots: Vec::new(),
                },
            }
        }

        pub(crate) fn set_texture_binding(&mut self, binding_group: u32, texture: TextureBindGroupSlot, sampler: Option<SamplerBindGroupSlot>) -> &mut Self {
            assert_ne!(self.test_data_binding_group, binding_group, "can't set test data binding group");

            let group = self.bind_groups.entry(binding_group).or_insert(Self::make_dummy_bind_group(binding_group, vec![], vec![], vec![]));

            group.texture_slots.slots.push(texture);
            
            if let Some(sampler) = sampler {
                group.sampler_slots.slots.push(sampler);   
            }
            
            self
        }

        pub(crate) fn set_storage_binding_group(&mut self, binding_group: u32, slots_to_stab: Vec<u32>, slots: Vec<DataBindGroupSlot>) -> &mut Self {
            assert_ne!(self.test_data_binding_group, binding_group, "can't set test data binding group");
            self.bind_groups.insert(binding_group, BindGroup {
                index: binding_group,
                storage_slots: SlotClass::<DataBindGroupSlot> {
                    dummy_slots: slots_to_stab.into_iter().collect(), slots,
                },
                sampler_slots: SlotClass::<SamplerBindGroupSlot> {
                    dummy_slots: HashSet::new(), slots: Vec::new(),
                },
                texture_slots: SlotClass::<TextureBindGroupSlot> {
                    dummy_slots: HashSet::new(), slots: Vec::new(),
                },
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

    pub(crate) struct GpuCodeExecutionContext {
    }

    impl GpuCodeExecutionContext {
        #[must_use]
        pub(crate) fn get(&self) -> Rc<GpuCodeExecutor> {
            Rc::new(GpuCodeExecutor::new())
        }

        #[must_use]
        pub(crate) fn new() -> Self {
            Self {}
        }
    }

    impl TestContext for GpuCodeExecutionContext {
        fn setup() -> GpuCodeExecutionContext {
            println!("Current thread ID: {:?}", thread::current().id());
            GpuCodeExecutionContext::new()
        }

        fn teardown(self) {
        }
    }

    pub(crate) struct GpuCodeExecutor {
        gpu_context: Rc<Context>,
        resources: Resources,
    }

    impl GpuCodeExecutor {
        #[must_use]
        pub(crate) fn new() -> Self {
            let context = create_headless_wgpu_vulkan_context();
            Self { gpu_context: context.clone(), resources: Resources::new(context) }
        }

        #[must_use]
        pub(crate) fn execute_code<TInput, TOutput>(&self, input: &[TInput], gpu_code: String, config: ExecutionConfig) -> Vec<TOutput>
        where
            TInput: Zeroable + Pod,
            TOutput: Zeroable + Pod
        {
            let device = self.gpu_context.device();

            let module = self.resources.create_shader_module("test GPU function execution", &gpu_code);
            let code = PipelineCode::new(module, seahash::hash(gpu_code.as_bytes()), "some_gpu_code".to_string());

            let input_buffer = self.resources.create_storage_buffer_write_only("input", bytemuck::cast_slice(input));
            let buffer_size = FrameBufferSize::new(input.len() as u32, 1);
            let mut output_buffer = DuplexLayer::<TOutput>::new(device, buffer_size, SupportUpdateFromCpu::Yes, "output");

            let mut pipeline_factory = PipelinesFactory::new(self.gpu_context.clone(), COMMON_PRESENTATION_FORMAT, None);

            let mut pipeline = ComputePipeline::new(pipeline_factory.create_compute_pipeline(config.entry_point, &code));
            pipeline.setup_bind_group(config.test_data_binding_group, Some("test data"), device, |bind_group| {
                bind_group.set_storage_entry(0, input_buffer.clone());
                bind_group.set_storage_entry(1, output_buffer.gpu_copy());
            });

            let universal_usage = BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::UNIFORM;
            let dummy_buffer = self.resources.create_buffer("dummy_buffer", universal_usage, &vec![0_u8; 256]);
            let dummy_sampler = self.resources.create_sampler("dummy_sampler");
            let dummy_texture = self.resources.create_texture("dummy_texture", 1, BitmapSize::new(1, 1));
            let dummy_texture_view = dummy_texture.create_view(&wgpu::TextureViewDescriptor::default());
            for (_, group) in config.bind_groups {
                pipeline.setup_bind_group(group.index, None, device, |bind_group| {
                    for slot in group.storage_slots.dummy_slots {
                        bind_group.set_storage_entry(slot, dummy_buffer.clone());
                    }
                    for slot in group.storage_slots.slots {
                        let buffer = self.resources.create_buffer("custom_buffer", universal_usage, &slot.data);
                        bind_group.set_storage_entry(slot.index, buffer.clone());
                    }

                    for slot in group.sampler_slots.dummy_slots {
                        bind_group.set_sampler_entry(slot, dummy_sampler.clone());
                    }
                    for slot in group.sampler_slots.slots {
                        bind_group.set_sampler_entry(slot.index, self.resources.create_sampler("custom_sampler"));
                    }

                    for slot in group.texture_slots.dummy_slots {
                        bind_group.set_texture_entry(slot, dummy_texture_view.clone());
                    }
                    for slot in group.texture_slots.slots {
                        let texture = self.resources.create_texture("dummy_texture", 1, BitmapSize::new(slot.size.x as usize, slot.size.y as usize));
                        self.resources.write_whole_srgba_texture_data(&texture, slot.data.as_ref());
                        bind_group.set_texture_entry(slot.index, texture.create_view(&wgpu::TextureViewDescriptor::default()));
                    }
                });
            }

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pipeline.set_into_pass(&mut pass);
                let workgroup_count = input.len().div_ceil(64);
                pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
            }
            output_buffer.prepare_cpu_read(&mut encoder);
            self.gpu_context.queue().submit(Some(encoder.finish()));

            let copy_wait = output_buffer.read_cpu_copy();
            self.gpu_context.wait(None);
            pollster::block_on(copy_wait);

            output_buffer.cpu_copy().clone()
        }
    }

    #[must_use]
    pub(crate) fn create_checkerboard_texture_data(
        width: u32,
        height: u32,
        checker_size: u32,
    ) -> Vec<u8> {
        assert_gt!(width, 0, "width must be greater than 0");
        assert_gt!(height, 0, "height must be greater than 0");
        assert_gt!(checker_size, 0, "checker_size must be greater than 0");

        let mut data: Vec<u8> = Vec::with_capacity((width * height) as usize * BYTES_IN_RGBA_QUARTET);

        let white = [255_u8, 255, 255, 255];
        let black = [  0_u8,   0,   0, 255];

        for y in 0..height {
            let checker_y = y / checker_size;

            for x in 0..width {
                let checker_x = x / checker_size;

                let is_white = (checker_x + checker_y) % 2 == 0;
                data.extend_from_slice(if is_white { &white } else { &black });
            }
        }

        data
    }
}