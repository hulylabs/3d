#[cfg(test)]
pub(crate) mod tests {
    use crate::gpu::render::WHOLE_TRACER_GPU_CODE;
    use crate::tests::scaffolding::dummy_implementations::tests::{DUMMY_IMPLEMENTATIONS, TEST_DATA_IO_BINDING_GROUP};
    use crate::tests::scaffolding::shader_entry_generator::tests::ShaderFunction;

    pub(crate) enum FieldKind {
        Vector4,
        Vector3,
        Vector2,
        Scalar,
    }

    #[must_use]
    pub(crate) fn make_shader_function(field_name_to_fetch: &str, field_kind: FieldKind, data_source: &str) -> ShaderFunction {
        let name = format!("fetch_{field_name_to_fetch}_data")
            .replace(".", "_")
            .replace("[", "_")
            .replace("]", "_")
            ;

        let body = match field_kind {
            FieldKind::Vector4 => {
                format!("fn {name}(index: f32) -> vec4f {{ return vec4f({data_source}.{field_name_to_fetch}); }}",)
            }
            FieldKind::Vector3 => {
                format!("fn {name}(index: f32) -> vec4f {{ return vec4f({data_source}.{field_name_to_fetch}.xyz, -7.0); }}",)
            }
            FieldKind::Vector2 => {
                format!("fn {name}(index: f32) -> vec4f {{ return vec4f(vec2f({data_source}.{field_name_to_fetch}.xy), 0.0, -7.0); }}",)
            }
            FieldKind::Scalar => {
                format!("fn {name}(index: f32) -> vec4f {{ return vec4f(f32({data_source}.{field_name_to_fetch}), 0.0, 0.0, -7.0); }}",)
            }
        };

        ShaderFunction::new("f32", "vec4f", name)
            .with_binding_group(TEST_DATA_IO_BINDING_GROUP)
            .with_additional_shader_code(WHOLE_TRACER_GPU_CODE)
            .with_additional_shader_code(DUMMY_IMPLEMENTATIONS)
            .with_additional_shader_code(body)
    }
}