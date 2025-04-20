#[cfg(test)]
mod tests {
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::render::CODE_FOR_GPU;
    use crate::gpu::resources::{Resources, ShaderCreationError};

    #[test]
    fn test_compilation() {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context, wgpu::TextureFormat::Rgba8Unorm);

        let shader = resources.create_shader_module("whole gpu code", CODE_FOR_GPU);

        shader.err().and_then(|error| -> Option<ShaderCreationError> {
            panic!("{}", error);
        });
    }
}
