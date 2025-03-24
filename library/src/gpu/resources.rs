use wgpu::util::DeviceExt;
use wgpu::TextureView;

// TODO: work in progress

pub(crate) struct Resources {
    device: wgpu::Device,
    queue: wgpu::Queue,
    presentation_format: wgpu::TextureFormat,
}

impl Resources {
    fn create_shader_module(&self, source: &str) -> wgpu::ShaderModule {
        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None, //TODO: name the shader
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    }

    pub fn create_uniform_buffer(&self, label: &str, uniform_array: &[u8]) -> (wgpu::Buffer, Vec<u8>) {
        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: uniform_array,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        self.queue.write_buffer(&uniform_buffer, 0, uniform_array);
        (uniform_buffer, uniform_array.to_vec())
    }

    pub fn create_storage_buffer_write_only(&self, label: &str, buffer_array: &[u8]) -> (wgpu::Buffer, Vec<u8>) {
        let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: buffer_array,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        self.queue.write_buffer(&storage_buffer, 0, buffer_array);
        (storage_buffer, buffer_array.to_vec())
    }

    pub fn create_storage_buffer_read_write(&self, label: &str, buffer_array: &[u8]) -> (wgpu::Buffer, Vec<u8>) {
        let storage_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: buffer_array,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });
        self.queue.write_buffer(&storage_buffer, 0, buffer_array);
        (storage_buffer, buffer_array.to_vec())
    }

    pub fn create_vertex_buffer(&self, label: &str, buffer_array: &[u8]) -> (wgpu::Buffer, Vec<u8>) {
        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: buffer_array,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        self.queue.write_buffer(&vertex_buffer, 0, buffer_array);
        (vertex_buffer, buffer_array.to_vec())
    }

    pub fn create_render_pipeline(&self, module: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"), //TODO: meaningfull label
            layout: Some(&self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs"),
                compilation_options: Default::default(), //TODO: what options are available?
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 2 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("fs"),
                compilation_options: Default::default(), //TODO: what options are available?
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.presentation_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None, // TODO: how can we use it?
        })
    }

    pub fn extract_frame_buffer_view(&self, surface: wgpu::Surface<'static>) -> TextureView {
        surface.get_current_texture().unwrap().texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn create_render_pass_descriptor(&self, frame_buffer: &TextureView, target: &mut wgpu::RenderPassDescriptor) {
        // TODO: this could not be implemented
        // wgpu::RenderPassDescriptor {
        //     label: Some("renderPass"),
        //     color_attachments: &[
        //         Some(wgpu::RenderPassColorAttachment {
        //             view: frame_buffer,
        //             resolve_target: None,
        //             ops: wgpu::Operations {
        //                 load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        //                 store: StoreOp::Store,
        //             },
        //         }),
        //     ],
        //     depth_stencil_attachment: None,
        //     occlusion_query_set: None,
        //     timestamp_writes: None, // TODO: what is this?
        // }
    }

    pub fn create_compute_pipeline(&self, module: &wgpu::ShaderModule) -> wgpu::ComputePipeline {
        self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            compilation_options: Default::default(), //TODO: what options are available?
            layout: Some(&self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),
            module,
            entry_point: Some("computeFrameBuffer"),
            cache: None, // TODO: how can be used?
        })
    }

    pub fn compute_pass(&self, compute_pipeline: &wgpu::ComputePipeline, bind_group_compute: &wgpu::BindGroup, work_groups_needed: u32) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("compute encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor
            {
                label: Some("compute pass"),
                timestamp_writes: None, // TODO: what can be used for?
            });

            pass.set_pipeline(compute_pipeline);
            pass.set_bind_group(0, bind_group_compute, &[]);
            pass.dispatch_workgroups(work_groups_needed, 1, 1);
        }
        let command_buffer = encoder.finish();
        self.queue.submit(Some(command_buffer));
    }

    pub fn render_pass(&self, render_pass_descriptor: &mut wgpu::RenderPassDescriptor, render_pipeline: &wgpu::RenderPipeline, bind_group: &wgpu::BindGroup, vertex_buffer: &wgpu::Buffer) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render encoder") });
        {
            let mut render_pass = encoder.begin_render_pass(render_pass_descriptor);
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }
        let render_command_buffer = encoder.finish();
        self.queue.submit(Some(render_command_buffer));
    }

    pub fn create_command_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }

    pub fn make_render_pass(&self, render_encoder: &mut wgpu::CommandEncoder, render_pass_descriptor: &wgpu::RenderPassDescriptor, render_pipeline: &wgpu::RenderPipeline, bind_group: &wgpu::BindGroup, vertex_buffer: &wgpu::Buffer, num_draw_calls: u32) {
        {
            let mut render_pass = render_encoder.begin_render_pass(render_pass_descriptor);
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..num_draw_calls, 0..1);
        }
    }

    pub fn add_command_buffer_to_queue(&self, render_encoder: wgpu::CommandEncoder) {
        self.queue.submit([render_encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use crate::{Engine, DEVICE_LABEL};
    use super::*;

    #[must_use]
    async fn create_wgpu_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .expect("failed to find an adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(DEVICE_LABEL),
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("failed to create device");

        (device, queue)
    }

    fn make_system_under_test() -> Resources {
        let (device, queue) = pollster::block_on(create_wgpu_device());
        Resources {
            device: device,
            queue: queue,
            presentation_format: wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    #[test]
    fn test_create_shader_module() {
    }
}