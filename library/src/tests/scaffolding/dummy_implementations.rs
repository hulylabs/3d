#[cfg(test)]
pub(crate) mod tests {
    pub(crate) const TEST_DATA_IO_BINDING_GROUP: u32 = 3;

    pub(crate) const DUMMY_IMPLEMENTATIONS: &str = include_str!("_dummy_implementations.wgsl");

    pub(crate) const DUMMY_TEXTURE_SELECTION: &str = "\
        fn procedural_texture_select(index: i32, position: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {\
        return vec3f(0.0);\
        }";
}