use crate::gpu::context::Context;
use std::rc::Rc;
use wgpu::util::DeviceExt;
use wgpu::BufferUsages;
// TODO: work in progress

pub(crate) struct Resources {
    context: Rc<Context>,
}

impl Resources {
    #[must_use]
    pub(crate) fn new(context: Rc<Context>) -> Self {
        Self { context }
    }

    #[must_use]
    pub(crate) fn create_shader_module(&self, label: &str, shader_source_code: &str) -> Rc<wgpu::ShaderModule> {
        Rc::new(
            self.context.device().create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(shader_source_code.into()), //TODO: can we use naga module created above?
            })
        )
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;

    #[must_use]
    fn make_system_under_test() -> Resources {
        Resources { context : create_headless_wgpu_context() }
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