#[cfg(test)]
pub(crate) mod tests {
    pub(crate) const COMMON_GPU_EVALUATIONS_EPSILON: f32 = 1e-5;
    
    pub(crate) const COMMON_PRESENTATION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
}