pub(crate) struct Context {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Context {
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self { device, queue }
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}
