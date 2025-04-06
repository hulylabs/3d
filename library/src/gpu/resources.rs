use crate::gpu::context::Context;
use std::rc::Rc;
use thiserror::Error;
use wgpu::util::DeviceExt;
use wgpu::{BufferAddress, BufferUsages, TextureView};

// TODO: work in progress

const FRAMEBUFFER_CHANNELS_COUNT: usize = 4;

#[derive(Error, Debug)]
pub(super) enum ShaderCreationError {
    #[error("shader '{shader_name:?}' parse error: {message:?}")]
    ParseError {
        shader_name: String,
        message: String,
    },
    #[error("shader '{shader_name:?}' validation error: {message:?}")]
    ValidationError {
        shader_name: String,
        message: String,
    },
}

pub(super) struct Resources {
    context: Rc<Context>,
    presentation_format: wgpu::TextureFormat,
}

impl Resources {

    pub fn new(context: Rc<Context>, presentation_format: wgpu::TextureFormat) -> Self {
        Self { context, presentation_format }
    }

    pub(super) fn create_shader_module(&self, shader_name: &str, shader_source_code: &str) -> Result<wgpu::ShaderModule, ShaderCreationError> {
        // TODO: this validation text output is unreadable =( but create_shader_module panics
        // let module = parse_str(shader_source_code).map_err(|e| ShaderCreationError::ParseError {
        //     shader_name: shader_name.to_string(),
        //     message: format!("{:?}", e),
        // })?;
        // let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        // validator.validate(&module).map_err(|e| ShaderCreationError::ValidationError {
        //     shader_name: shader_name.to_string(),
        //     message: format!("{:?}", e),
        // })?;
        Ok(self.context.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(shader_name),
            source: wgpu::ShaderSource::Wgsl(shader_source_code.into()), //TODO: can we use naga module created above?
        }))
    }

    fn create_buffer(&self, label: &str, usage: BufferUsages, buffer_data: &[u8]) -> wgpu::Buffer {
        let buffer = self.context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: buffer_data,
            usage,
        });
        self.context.queue().write_buffer(&buffer, 0, buffer_data);

        buffer
    }

    pub(super) fn create_uniform_buffer(&self, label: &str, buffer_data: &[u8]) -> wgpu::Buffer {
        self.create_buffer(label, BufferUsages::UNIFORM | BufferUsages::COPY_DST, buffer_data)
    }

    pub(super) fn create_storage_buffer_write_only(&self, label: &str, buffer_data: &[u8]) -> wgpu::Buffer {
        self.create_buffer(label, BufferUsages::STORAGE | BufferUsages::COPY_DST, buffer_data)
    }

    pub(super) fn create_frame_buffer(&self, width: u32, height: u32) -> wgpu::Buffer {
        let size_bytes = (width * height) as usize * FRAMEBUFFER_CHANNELS_COUNT * size_of::<f32>();
        self.context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame buffer"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
            size: size_bytes as BufferAddress,
            mapped_at_creation: false,
        })
    }

    pub(super) fn create_storage_buffer_read_write(&self, label: &str, buffer_data: &[u8]) -> wgpu::Buffer {
        self.create_buffer(label, BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST, buffer_data)
    }

    pub(super) fn create_vertex_buffer(&self, label: &str, buffer_data: &[u8]) -> wgpu::Buffer {
        self.create_buffer(label, BufferUsages::VERTEX | BufferUsages::COPY_DST, buffer_data)
    }

    pub(super) fn create_rasterization_pipeline(&self, module: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
        self.context.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rasterization pipeline"),
            layout: None,
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

    pub(super) fn extract_frame_buffer_view(&self, surface: wgpu::Surface<'static>) -> TextureView {
        surface.get_current_texture().unwrap().texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub(super) fn create_compute_pipeline(&self, module: &wgpu::ShaderModule) -> wgpu::ComputePipeline {
        self.context.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ray tracing compute pipeline"),
            compilation_options: Default::default(), //TODO: what options are available?
            layout: None,
            module,
            entry_point: Some("computeFrameBuffer"),
            cache: None, // TODO: how can be used?
        })
    }

    pub(super) fn compute_pass(&self, compute_pipeline: &wgpu::ComputePipeline, bind_group_compute: &wgpu::BindGroup, work_groups_needed: u32) {
        let mut encoder = self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("compute encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor
            {
                label: Some("ray tracing compute pass"),
                timestamp_writes: None, // TODO: what can be used for?
            });

            pass.set_pipeline(compute_pipeline);
            pass.set_bind_group(0, bind_group_compute, &[]);
            pass.dispatch_workgroups(work_groups_needed, 1, 1);
        }
        let command_buffer = encoder.finish();
        self.context.queue().submit(Some(command_buffer));
    }

    pub(super) fn render_pass(&self, render_pass_descriptor: &mut wgpu::RenderPassDescriptor, render_pipeline: &wgpu::RenderPipeline, bind_group: &wgpu::BindGroup, vertex_buffer: &wgpu::Buffer) {
        let mut encoder = self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render encoder") });
        {
            let mut render_pass = encoder.begin_render_pass(render_pass_descriptor);
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }
        let render_command_buffer = encoder.finish();
        self.context.queue().submit(Some(render_command_buffer));
    }

    pub(super) fn create_command_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }

    pub(super) fn add_command_buffer_to_queue(&self, render_encoder: wgpu::CommandEncoder) {
        self.context.queue().submit([render_encoder.finish()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_device;

    #[must_use]
    fn make_system_under_test() -> Resources {
        let context = pollster::block_on(create_headless_wgpu_device());
        Resources {
            context: Rc::new(context),
            presentation_format: wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    const TRIVIAL_SHADER_CODE: &str = r#"
        @vertex
        fn vs_main() -> @builtin(position) vec4<f32> {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    "#;

    const SHADER_CODE_WITH_SYNTAX_ERROR: &str = r#"
        @vertex
        fn vs_main() -> @builtin(position) vec4<f32> {
            return 1.0);
        }
    "#;

    const SHADER_CODE_WITH_VALIDATION_ERROR: &str = r#"
        @vertex
        fn vs_main() -> @builtin(position) vec4<f32> {
            return vec3<f32>(0, 0, 0, 1);
        }
    "#;

    const DUMMY_BYTE_ARRAY: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    #[test]
    fn test_create_shader_module_successful_compilation() {
        let system_under_test = make_system_under_test();

        let shader = system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), TRIVIAL_SHADER_CODE);

        shader.err().and_then(|error| -> Option<ShaderCreationError>{
            panic!("{}", error);
        });
    }

    #[test]
    fn test_create_shader_module_syntax_error_compilation() {
        let system_under_test = make_system_under_test();

        let shader = system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), SHADER_CODE_WITH_SYNTAX_ERROR);

        match shader {
            Ok(_) => {
                panic!("shader compilation expected to fail");
            }
            Err(error) => {
                match error {
                    ShaderCreationError::ParseError { .. } => {}
                    ShaderCreationError::ValidationError { .. } => {
                        panic!("parse error expected");
                    }
                }
            }
        }
    }

    #[test]
    fn test_create_shader_module_validation_error_compilation() {
        let system_under_test = make_system_under_test();

        let shader = system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), SHADER_CODE_WITH_VALIDATION_ERROR);

        match shader {
            Ok(_) => {
                panic!("shader compilation expected to fail");
            }
            Err(error) => {
                match error {
                    ShaderCreationError::ParseError { .. } => {
                        panic!("validation error expected");
                    }
                    ShaderCreationError::ValidationError { .. } => {}
                }
            }
        }
    }

    #[test]
    fn test_create_uniform_buffer() {
        let system_under_test = make_system_under_test();

        let buffer = system_under_test.create_uniform_buffer(
            concat!("unit tests: buffer ", file!(), ", line: ", line!()), &DUMMY_BYTE_ARRAY);

        // TODO: do we need to wait for the queue to finish write? guess, yes
        // TODO: system_under_test.queue.submit([]); - this will initiate the actual data transfer on GPU

        assert_eq!(buffer.usage(), BufferUsages::UNIFORM | BufferUsages::COPY_DST);
    }

    #[test]
    fn test_create_storage_buffer_write_only() {
        let system_under_test = make_system_under_test();

        let buffer = system_under_test.create_storage_buffer_write_only(
            concat!("unit tests: buffer ", file!(), ", line: ", line!()), &DUMMY_BYTE_ARRAY);

        assert_eq!(buffer.usage(), BufferUsages::STORAGE | BufferUsages::COPY_DST);
    }

    #[test]
    fn test_create_storage_buffer_read_write() {
        let system_under_test = make_system_under_test();

        let buffer = system_under_test.create_storage_buffer_read_write(
            concat!("unit tests: buffer ", file!(), ", line: ", line!()), &DUMMY_BYTE_ARRAY);

        assert_eq!(buffer.usage(), BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST);
    }

    #[test]
    fn test_create_vertex_buffer() {
        let system_under_test = make_system_under_test();

        let buffer = system_under_test.create_vertex_buffer(
            concat!("unit tests: buffer ", file!(), ", line: ", line!()), &DUMMY_BYTE_ARRAY);

        assert_eq!(buffer.usage(), BufferUsages::VERTEX | BufferUsages::COPY_DST);
    }

}