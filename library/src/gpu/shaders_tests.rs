#[cfg(test)]
mod tests {
    use crate::gpu::headless_device::create_headless_wgpu_device;
    use crate::gpu::render::CODE_FOR_GPU;
    use crate::gpu::resources::{Resources, ShaderCreationError};
    use std::rc::Rc;

    #[test]
    fn test_compilation() {
        let context = Rc::new(pollster::block_on(create_headless_wgpu_device()));
        let resources = Resources::new(context, wgpu::TextureFormat::Rgba8Unorm);

        let shader = resources.create_shader_module("whole gpu code", CODE_FOR_GPU);

        shader.err().and_then(|error| -> Option<ShaderCreationError> {
            panic!("{}", error);
        });
    }
}
