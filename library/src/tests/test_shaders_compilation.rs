﻿#[cfg(test)]
mod tests {
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
    use crate::gpu::resources::Resources;
    use crate::sdf::framework::shader_code::format_sdf_selection_function_opening;

    #[test]
    fn test_compilation() {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context);

        let dummy_sdf_selection = format!("{}return 0.0; }}", format_sdf_selection_function_opening());
        let whole_shader_code = format!("{}\n{}", WHOLE_TRACER_GPU_CODE, dummy_sdf_selection); 
        
        let _ = resources.create_shader_module("whole gpu code", whole_shader_code.as_str());
    }
}
