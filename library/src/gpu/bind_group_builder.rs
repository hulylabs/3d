use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{BindGroup, BindingResource, Buffer, Sampler, TextureView};

pub(crate) struct BindGroupBuilder<'a> {
    index: u32,
    label: Option<&'a str>,
    layout: wgpu::BindGroupLayout,

    accumulated_storage_entries: HashMap<u32, Rc<Buffer>>,
    accumulated_sampler_entries: HashMap<u32, Sampler>,
    accumulated_texture_entries: HashMap<u32, TextureView>,
}

impl<'a> BindGroupBuilder<'a> {
    #[must_use]
    pub(super) fn new(index: u32, label: Option<&'a str>, layout: wgpu::BindGroupLayout) -> Self {
        Self {
            index,
            label,
            layout,
            accumulated_storage_entries: HashMap::new(),
            accumulated_sampler_entries: HashMap::new(),
            accumulated_texture_entries: HashMap::new(),
        }
    }

    pub(crate) fn set_storage_entry(&mut self, slot: u32, buffer: Rc<Buffer>) -> &mut Self {
        assert_eq!(self.accumulated_sampler_entries.contains_key(&slot), false, "slot already occupied by a sampler");
        assert_eq!(self.accumulated_texture_entries.contains_key(&slot), false, "slot already occupied by a texture");

        let previous = self.accumulated_storage_entries.insert(slot, buffer);
        assert!(previous.is_none(), "slot {slot} already set");
        self
    }

    pub(crate) fn set_sampler_entry(&mut self, slot: u32, sampler: Sampler) -> &mut Self {
        assert_eq!(self.accumulated_storage_entries.contains_key(&slot), false, "slot already occupied by a storage buffer");
        assert_eq!(self.accumulated_texture_entries.contains_key(&slot), false, "slot already occupied by a texture");

        let previous = self.accumulated_sampler_entries.insert(slot, sampler);
        assert!(previous.is_none(), "slot {slot} already set");
        self
    }

    pub(crate) fn set_texture_entry(&mut self, slot: u32, sampler: TextureView) -> &mut Self {
        assert_eq!(self.accumulated_storage_entries.contains_key(&slot), false, "slot already occupied by a storage buffer");
        assert_eq!(self.accumulated_sampler_entries.contains_key(&slot), false, "slot already occupied by a sampler");

        let previous = self.accumulated_texture_entries.insert(slot, sampler);
        assert!(previous.is_none(), "slot {slot} already set");
        self
    }

    #[must_use]
    pub(super) fn make_bind_group(&self, device: &wgpu::Device) -> BindGroup {
        let entries = {
            let mut entries = Vec::new();
            self.accumulated_storage_entries.iter().for_each(
                |(slot_number, buffer)| {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot_number,
                        resource: buffer.as_entire_binding(),
                    });
                });
            self.accumulated_sampler_entries.iter().for_each(
                |(slot_number, sampler)| {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot_number,
                        resource: BindingResource::Sampler(sampler),
                    });
                });
            self.accumulated_texture_entries.iter().for_each(
                |(slot_number, texture_view)| {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot_number,
                        resource: BindingResource::TextureView(texture_view),
                    });
                });
            entries
        };
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: &self.layout,
            entries: entries.as_slice(),
        })
    }

    #[must_use]
    pub(super) fn index(&self) -> u32 {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::context::Context;
    use crate::gpu::headless_device::tests::create_headless_wgpu_vulkan_context;
    use crate::gpu::resources::Resources;
    use test_context::{test_context, TestContext};
    use wgpu::BindGroupLayoutEntry;
    use wgpu::wgt::PollType;

    #[must_use]
    fn describe_common_storage_buffer() -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    struct TestFixture {
        context: Rc<Context>,
        resources: Resources,
        bind_group_layout: wgpu::BindGroupLayout,
        compute_pipeline: wgpu::ComputePipeline,
    }

    impl TestContext for TestFixture {
        fn setup() -> Self {
            let context = create_headless_wgpu_vulkan_context();
            let resources = Resources::new(context.clone());

            let bind_group_layout = context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("test-bind-group-layout"),
                entries: &[
                    describe_common_storage_buffer()
                    ,
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    }
                    ,
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    }
                    ,
                ],
            });

            let pipeline_layout = context.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("test-pipeline-layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let shader_module = resources.create_shader_module("test-shader", r#"
                @group(0) @binding(0) var<storage, read_write> data: array<f32>;
                @group(0) @binding(1) var test_sampler: sampler;
                @group(0) @binding(2) var test_texture: texture_2d<f32>;

                @compute @workgroup_size(1)
                fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
                    data[global_id.x] = textureSampleLevel(test_texture, test_sampler, vec2f(0.0, 0.0), 0.0).r;
                }
            "#);

            let compute_pipeline = context.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("test-compute-pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: None,
                compilation_options: Default::default(),
                cache: None,
            });

            TestFixture { 
                context, 
                resources, 
                bind_group_layout: bind_group_layout.clone(),
                compute_pipeline,
            }
        }

        fn teardown(self) {}
    }

    impl TestFixture {
        #[must_use]
        fn create_test_buffer(&self) -> Rc<Buffer> {
            self.resources.create_storage_buffer_write_only("test-buffer", &[42u8; 256])
        }

        #[must_use]
        fn create_test_sampler(&self) -> Sampler {
            self.context.device().create_sampler(&wgpu::SamplerDescriptor {
                label: Some("test-sampler"),
                ..Default::default()
            })
        }

        #[must_use]
        fn create_test_texture_view(&self) -> TextureView {
            let texture = self.context.device().create_texture(&wgpu::TextureDescriptor {
                label: Some("test-texture"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1, },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        }

        fn execute_compute_with_bind_group(&self, bind_group: &BindGroup) {
            let mut encoder = self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test-command-encoder"),
            });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("test-compute-pass"),
                    timestamp_writes: None,
                });
                
                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, bind_group, &[]);
                compute_pass.dispatch_workgroups(1, 1, 1);
            }

            let submission = self.context.queue().submit(std::iter::once(encoder.finish()));
            
            let poll = self.context.device().poll(PollType::WaitForSubmissionIndex(submission));
            poll.expect("failed to wait for submission");
        }
    }

    #[test_context(TestFixture)]
    #[test]
    fn test_set_entry(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());
        let test_buffer = fixture.create_test_buffer();
        let test_sampler = fixture.create_test_sampler();
        let test_texture = fixture.create_test_texture_view();
        
        system_under_test
            .set_storage_entry(0, test_buffer)
            .set_sampler_entry(1, test_sampler)
            .set_texture_entry(2, test_texture);
        let bind_group = system_under_test.make_bind_group(fixture.context.device());

        fixture.execute_compute_with_bind_group(&bind_group);
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot 0 already set")]
    fn test_set_storage_entry_duplicate_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_storage_entry(0, fixture.create_test_buffer())
            .set_storage_entry(0, fixture.create_test_buffer());
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot 0 already set")]
    fn test_set_sampler_entry_duplicate_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_sampler_entry(0, fixture.create_test_sampler())
            .set_sampler_entry(0, fixture.create_test_sampler());
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot 0 already set")]
    fn test_set_texture_entry_duplicate_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_texture_entry(0, fixture.create_test_texture_view())
            .set_texture_entry(0, fixture.create_test_texture_view());
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot already occupied by a storage buffer")]
    fn test_set_sampler_on_storage_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_storage_entry(0, fixture.create_test_buffer())
            .set_sampler_entry(0, fixture.create_test_sampler());
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot already occupied by a sampler")]
    fn test_set_storage_on_sampler_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_sampler_entry(0, fixture.create_test_sampler())
            .set_storage_entry(0, fixture.create_test_buffer());
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot already occupied by a texture")]
    fn test_set_storage_on_texture_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_texture_entry(0, fixture.create_test_texture_view())
            .set_storage_entry(0, fixture.create_test_buffer());
    }

    #[test_context(TestFixture)]
    #[test]
    #[should_panic(expected = "slot already occupied by a texture")]
    fn test_set_sampler_on_texture_slot(fixture: &TestFixture) {
        let mut system_under_test = BindGroupBuilder::new(0, None, fixture.bind_group_layout.clone());

        system_under_test
            .set_texture_entry(0, fixture.create_test_texture_view())
            .set_sampler_entry(0, fixture.create_test_sampler());
    }

    #[test_context(TestFixture)]
    #[test]
    fn test_make_bind_group_empty_layout(fixture: &TestFixture) {
        let empty_layout = fixture.context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("empty-layout"),
            entries: &[],
        });
        
        let system_under_test = BindGroupBuilder::new(0, None, empty_layout);
        
        let bind_group = system_under_test.make_bind_group(fixture.context.device());

        // we can't directly test the bind group contents, but we can verify it doesn't panic when created
        assert!(!std::ptr::eq(&bind_group as *const _, std::ptr::null()));
    }

    #[test_context(TestFixture)]
    #[test]
    fn test_make_bind_group_with_storage_buffer_only(fixture: &TestFixture) {
        let storage_only_layout = fixture.context.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("storage-only-layout"),
            entries: &[
                describe_common_storage_buffer(),
            ],
        });

        let mut system_under_test = BindGroupBuilder::new(0, Some("storage-only"), storage_only_layout);
        let test_buffer = fixture.create_test_buffer();
        
        system_under_test.set_storage_entry(0, test_buffer);
        
        let bind_group = system_under_test.make_bind_group(fixture.context.device());

        // we can't directly test the bind group contents, but we can verify it doesn't panic when created
        assert!(!std::ptr::eq(&bind_group as *const _, std::ptr::null()));
    }

    #[test_context(TestFixture)]
    #[test]
    fn test_index_get(fixture: &TestFixture) {
        let expected_index = 123;
        let system_under_test = BindGroupBuilder::new(expected_index, None, fixture.bind_group_layout.clone());
        
        assert_eq!(system_under_test.index(), expected_index);
    }
}