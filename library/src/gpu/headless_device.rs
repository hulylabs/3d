#[cfg(test)]
pub(crate) mod tests {
    use crate::gpu::context::Context;

    const HEADLESS_DEVICE_LABEL: &str = "Rust Tracer Library Headless Device";

    #[must_use]
    pub(crate) async fn create_headless_wgpu_device() -> Context {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .expect("failed to find an adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(HEADLESS_DEVICE_LABEL),
                    required_features: wgpu::Features::default(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("failed to create device");

        Context::new(device, queue)
    }
}