pub(crate) struct Context {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Context {
    #[must_use]
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self { device, queue }
    }

    #[must_use]
    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}
