#[cfg(test)]
mod tests {
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
    use crate::gpu::resources::Resources;
    use crate::material::texture_shader_code::write_texture_selection_function_opening;
    use crate::sdf::framework::sdf_shader_code::format_sdf_selection_function_opening;

    #[must_use]
    fn format_texture_selection_function_opening() -> String {
        let mut result: String = String::new();
        write_texture_selection_function_opening(&mut result).expect("failed to format procedural texture selection opening");
        result
    }
    
    #[test]
    fn test_compilation() {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context);

        let dummy_sdf_selection = format!("{}{{\nreturn 0.0;}}", format_sdf_selection_function_opening());
        let dummy_procedural_texture_selection = format!("{}{{\nreturn vec3f(0.0);}}", format_texture_selection_function_opening());
        
        let whole_shader_code = format!("{}\n{}\n{}", WHOLE_TRACER_GPU_CODE, dummy_sdf_selection, dummy_procedural_texture_selection); 
        
        let _ = resources.create_shader_module("whole gpu code", whole_shader_code.as_str());
    }
}
