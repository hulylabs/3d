use crate::gpu::context::Context;
use std::rc::Rc;
use wgpu::util::DeviceExt;
use wgpu::BufferUsages;

// TODO: work in progress

pub(crate) struct Resources {
    context: Rc<Context>,
    presentation_format: wgpu::TextureFormat,
}

impl Resources {
    #[must_use]
    pub(crate) fn new(context: Rc<Context>, presentation_format: wgpu::TextureFormat) -> Self {
        Self { context, presentation_format }
    }

    #[must_use]
    pub(crate) fn create_shader_module(&self, shader_name: &str, shader_source_code: &str) -> wgpu::ShaderModule {
        self.context.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(shader_name),
            source: wgpu::ShaderSource::Wgsl(shader_source_code.into()), //TODO: can we use naga module created above?
        })
    }

    #[must_use]
    pub(crate) fn create_buffer(&self, label: &str, usage: BufferUsages, buffer_data: &[u8]) -> Rc<wgpu::Buffer> {
        let buffer = self.context.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: buffer_data,
            usage,
        });
        self.context.queue().write_buffer(&buffer, 0, buffer_data);

        Rc::new(buffer)
    }

    #[must_use]
    pub(super) fn create_uniform_buffer(&self, label: &str, buffer_data: &[u8]) -> Rc<wgpu::Buffer> {
        self.create_buffer(label, BufferUsages::UNIFORM | BufferUsages::COPY_DST, buffer_data)
    }

    #[must_use]
    pub(crate) fn create_storage_buffer_write_only(&self, label: &str, buffer_data: &[u8]) -> Rc<wgpu::Buffer> {
        self.create_buffer(label, BufferUsages::STORAGE | BufferUsages::COPY_DST, buffer_data)
    }

    #[must_use]
    pub(super) fn create_rasterization_pipeline(&self, module: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
        self.context.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rasterization pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[], // full screen quad vertices specified as a const in the shader
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: None,
                compilation_options: Default::default(),
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
    
    #[must_use]
    pub(crate) fn create_compute_pipeline(&self, routine: ComputeRoutineEntryPoint, module: &wgpu::ShaderModule) -> wgpu::ComputePipeline {
        self.context.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: routine.name(),
            compilation_options: Default::default(),
            layout: None,
            module,
            entry_point: routine.name(),
            cache: None, // TODO: how can be used?
        })
    }
}

pub(crate) enum ComputeRoutineEntryPoint {
    ShaderObjectId,
    
    ShaderRayTracingMonteCarlo,
    ShaderRayTracingDeterministic,
    
    #[cfg(test)] Default,
    #[cfg(test)] TestDefault,
}

impl ComputeRoutineEntryPoint {
    fn name(&self) -> Option<&'static str> {
        match self {
            ComputeRoutineEntryPoint::ShaderObjectId => Some("compute_object_id_buffer"),
            ComputeRoutineEntryPoint::ShaderRayTracingMonteCarlo => Some("compute_color_buffer_monte_carlo"),
            ComputeRoutineEntryPoint::ShaderRayTracingDeterministic => Some("compute_color_buffer_deterministic"),
            #[cfg(test)] ComputeRoutineEntryPoint::TestDefault => Some("test_entry_point"),
            #[cfg(test)] ComputeRoutineEntryPoint::Default => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;

    #[must_use]
    fn make_system_under_test() -> Resources {
        Resources {
            context : create_headless_wgpu_context(),
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

        let _ = system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), TRIVIAL_SHADER_CODE);
    }

    #[test]
    #[should_panic]
    fn test_create_shader_module_syntax_error_compilation() {
        let system_under_test = make_system_under_test();

        let _ = system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), SHADER_CODE_WITH_SYNTAX_ERROR);
    }

    #[test]
    #[should_panic]
    fn test_create_shader_module_validation_error_compilation() {
        let system_under_test = make_system_under_test();

        let _ = system_under_test.create_shader_module(
            concat!("unit tests: file ", file!(), ", line: ", line!()), SHADER_CODE_WITH_VALIDATION_ERROR);
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
}