pub(crate) struct Context {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline_caching_supported: bool,
}

impl Context {
    #[must_use]
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue, pipeline_caching_supported: bool,) -> Self {
        Self { device, queue, pipeline_caching_supported, }
    }

    #[must_use]
    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[must_use]
    pub(crate) fn pipeline_caching_supported(&self) -> bool {
        self.pipeline_caching_supported
    }
}
