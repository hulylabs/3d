use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::object_id_layer::ObjectIdLayer;
use crate::gpu::output::utils::{FrameBufferLayerParametersBuilder, create_frame_buffer_layer};
use std::rc::Rc;
use wgpu::{Buffer, BufferUsages};

pub(crate) struct FrameBuffer {
    object_id: ObjectIdLayer,
    last_read_object_id: Vec<u32>,
    pixel_color: Rc<Buffer>,
}

impl FrameBuffer {
    const PIXELS_BUFFER_CHANNELS_COUNT: u32 = 4;
    
    const LABEL_PIXEL_COLOR_LAYER: &'static str = "pixel color buffer";
    
    #[must_use]
    pub(crate) fn new(device: &wgpu::Device, frame_buffer_size: FrameBufferSize) -> Self {
        let parameters_for_pixel_color = FrameBufferLayerParametersBuilder::new(BufferUsages::STORAGE)
            .label(Self::LABEL_PIXEL_COLOR_LAYER)
            .frame_buffer_size(frame_buffer_size)
            .bytes_per_channel(size_of::<f32>() as u32)
            .channels_count(Self::PIXELS_BUFFER_CHANNELS_COUNT)
            .build();

        Self {
            object_id: ObjectIdLayer::new(device, frame_buffer_size),
            pixel_color: Rc::new(create_frame_buffer_layer(device, &parameters_for_pixel_color)),
            last_read_object_id: Vec::new(),
        }
    }

    pub(crate) fn prepare_object_id_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.object_id.issue_copy_to_staging(encoder)
    }

    pub(crate) fn copy_object_id_from_gpu(&mut self) -> impl Future<Output = ()> {
        self.object_id.read_staging(|data| {
            self.last_read_object_id.clear();
            self.last_read_object_id.extend(data);
        })
    }

    #[must_use]
    pub(crate) fn pixel_color(&self) -> Rc<Buffer> {
        self.pixel_color.clone()
    }

    #[must_use]
    pub(crate) fn object_id_at_gpu(&self) -> Rc<Buffer> {
        self.object_id.gpu_render_target()
    }

    #[must_use]
    pub fn object_id_at_cpu(&self) -> &Vec<u32> {
        &self.last_read_object_id
    }

    pub(crate) fn invalidate(&mut self) {
        self.last_read_object_id.clear();
    }
}

#[cfg(test)]
mod tests {
    use wgpu::CommandEncoderDescriptor;
    use wgpu::wgt::PollType;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use super::*;

    #[must_use]
    fn test_buffer_size() -> FrameBufferSize {
        FrameBufferSize::new(801, 601)
    }
    
    #[test]
    fn test_construction() {
        let context = create_headless_wgpu_context();
        
        let system_under_test = FrameBuffer::new(context.device(), test_buffer_size());
        
        assert!(system_under_test.object_id_at_cpu().is_empty());
    }

    #[test]
    fn test_object_id_acquiring() {
        let context = create_headless_wgpu_context();

        let mut system_under_test = FrameBuffer::new(context.device(), test_buffer_size());

        let mut encoder = context.device().create_command_encoder(&CommandEncoderDescriptor { label: None });
        system_under_test.prepare_object_id_copy_from_gpu(&mut encoder);
        context.queue().submit(Some(encoder.finish()));

        let gpu_to_cpu_copy = system_under_test.copy_object_id_from_gpu();
        context.device().poll(PollType::Wait).expect("failed to poll the device");
        pollster::block_on(gpu_to_cpu_copy);

        assert_eq!(system_under_test.object_id_at_cpu().len(), test_buffer_size().area() as usize);
    }
}
