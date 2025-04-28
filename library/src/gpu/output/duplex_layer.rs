use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::frame_buffer_layer::FrameBufferLayer;
use bytemuck::AnyBitPattern;
use std::rc::Rc;

pub(super) struct DuplexLayer<T: Sized + AnyBitPattern> {
    gpu_located_part: FrameBufferLayer<T>,
    last_read: Vec<T>,
}

impl<T: Sized + AnyBitPattern> DuplexLayer<T> {
    #[must_use]
    pub(super) fn new(device: &wgpu::Device, frame_buffer_size: FrameBufferSize, marker: &str) -> Self {
        Self {
            gpu_located_part: FrameBufferLayer::<T>::new(device, frame_buffer_size, marker),
            last_read: Vec::new(),
        }
    }

    pub(crate) fn prepare_cpu_read(&self, encoder: &mut wgpu::CommandEncoder) {
        self.gpu_located_part.issue_copy_to_cpu_mediator(encoder)
    }
    
    pub(crate) fn read_cpu_copy(&mut self) -> impl Future<Output = ()> {
        self.gpu_located_part.read_cpu_mediator(|data| {
            self.last_read.clear();
            self.last_read.extend(data);
        })
    }
    
    #[must_use]
    pub(super) fn cpu_copy(&self) -> &Vec<T> {
        &self.last_read
    }

    #[must_use]
    pub(super) fn gpu_copy(&self) -> Rc<wgpu::Buffer> {
        self.gpu_located_part.gpu_render_target()
    }

    pub(super) fn invalidate_cpu_copy(&mut self) {
        self.last_read.clear();
    }
}
