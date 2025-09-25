#[must_use]
pub(crate) fn backend_vulkan_or_primary() -> wgpu::Backends {
    if wgpu::Instance::enabled_backend_features().contains(wgpu::Backends::VULKAN) {
        wgpu::Backends::VULKAN
    } else {
        wgpu::Backends::PRIMARY
    }
}
