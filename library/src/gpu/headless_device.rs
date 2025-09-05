#[cfg(test)]
pub(crate) mod tests {
    use crate::gpu::adapter_features::AdapterFeatures;
    use crate::gpu::context::Context;
    use std::rc::Rc;
    use std::sync::OnceLock;
    use wgpu::{Instance, Trace};

    const HEADLESS_DEVICE_LABEL: &str = "Rust Tracer Library Headless Device";

    /*
    Why a singleton, given that it’s one of the worst anti-patterns?

    In theory, each test could create its own instance. In practice,
    it turned out that on Windows, when tests are run massively in
    parallel, they can’t always get a separate VULKAN instance. If the
    implementation type is specified explicitly, some tests fail when
    trying to obtain it. But if we request a PRIMARY one, then some
    tests end up running under DirectX, where we have stability issues.

    That’s why we use exactly one explicitly chosen instance for all tests.
    */
    static VULKAN_INSTANCE: OnceLock<Instance> = OnceLock::new();

    #[must_use]
    pub(crate) fn get_vulkan_instance() -> &'static Instance {
        VULKAN_INSTANCE.get_or_init(|| {
            Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::VULKAN,
                ..Default::default()
            })
        })
    }

    #[must_use]
    pub(crate) fn create_headless_wgpu_vulkan_context() -> Rc<Context> {
        Rc::new(pollster::block_on(create_headless_wgpu_device_async(get_vulkan_instance())))
    }

    #[must_use]
    pub(crate) async fn create_headless_wgpu_device_async(instance: &Instance) -> Context {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                ..Default::default()
            })
            .await
            .expect("failed to find an adapter");

        let adapter_info = adapter.get_info();
        println!(
            "Adapter Info:\n\
            Name: {}\n\
            Backend: {:?}\n\
            Device Type: {:?}",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device_type,
        );
        
        let features = AdapterFeatures::new(&adapter);
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some(HEADLESS_DEVICE_LABEL),
                    required_features: features.desired_features(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: Trace::Off,
                },
            )
            .await
            .expect("failed to create device");

        Context::new(device, queue, features.pipeline_caching_supported(), adapter_info,)
    }
}