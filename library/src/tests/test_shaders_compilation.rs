#[cfg(test)]
mod tests {
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
    use crate::gpu::resources::Resources;

    #[test]
    fn test_compilation() {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context);
        
        const DUMMY_IMPLEMENTATIONS: &str = include_str!("../../assets/shaders/dummy_implementations.wgsl");
        let whole_shader_code = format!("{}\n{}", WHOLE_TRACER_GPU_CODE, DUMMY_IMPLEMENTATIONS);
        
        let _ = resources.create_shader_module("whole gpu code", whole_shader_code.as_str());
    }
}
