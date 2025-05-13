#[cfg(test)]
mod tests {
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::render::CODE_FOR_GPU;
    use crate::gpu::resources::Resources;
    use crate::sdf::shader_code::{format_sdf_selection_function_opening, SHADER_RETURN_KEYWORD};

    #[test]
    fn test_compilation() {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context, wgpu::TextureFormat::Rgba8Unorm);

        let dummy_sdf_selection = format!("{}{} 0.0; }}", format_sdf_selection_function_opening(), SHADER_RETURN_KEYWORD);
        let whole_shader_code = format!("{}\n{}", CODE_FOR_GPU, dummy_sdf_selection); 
        
        let _ = resources.create_shader_module("whole gpu code", whole_shader_code.as_str());
    }
}
