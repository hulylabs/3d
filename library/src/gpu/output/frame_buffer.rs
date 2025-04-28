use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::duplex_layer::DuplexLayer;
use crate::serialization::pod_vector::PodVector;
use std::rc::Rc;
use wgpu::Buffer;

pub(crate) struct FrameBuffer {
    object_id: DuplexLayer<u32>,
    albedo: DuplexLayer<PodVector>,
    normal: DuplexLayer<PodVector>,

    pixel_color: DuplexLayer<PodVector>,
}

impl FrameBuffer {
    #[must_use]
    pub(crate) fn new(device: &wgpu::Device, frame_buffer_size: FrameBufferSize) -> Self {
        Self {
            object_id: DuplexLayer::new(device, frame_buffer_size, "object id"),
            albedo: DuplexLayer::new(device, frame_buffer_size, "albedo"),
            normal: DuplexLayer::new(device, frame_buffer_size, "normal"),

            pixel_color: DuplexLayer::new(device, frame_buffer_size, "pixel color"),
        }
    }

    pub(crate) fn prepare_pixel_color_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.pixel_color.prepare_cpu_read(encoder);
    }
    
    pub(crate) fn prepare_object_id_and_normal_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.object_id.prepare_cpu_read(encoder);
        self.normal.prepare_cpu_read(encoder);
    }
    
    pub(crate) fn prepare_albedo_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.albedo.prepare_cpu_read(encoder);
    }

    #[must_use]
    pub(crate) fn copy_all_from_gpu(&mut self) -> impl Future<Output = ()> {
        let object_id_read = self.object_id.read_cpu_copy();
        let normals_read = self.normal.read_cpu_copy();
        let albedo_read = self.albedo.read_cpu_copy();
        
        async move {
            futures::join!(object_id_read, normals_read, albedo_read);
            ()
        }
    }

    #[must_use]
    pub(crate) fn copy_pixel_colors_from_gpu(&mut self) -> impl Future<Output = ()> {
        self.pixel_color.read_cpu_copy()
    }
    
    #[must_use]
    pub(crate) fn copy_albedo_from_gpu(&mut self) -> impl Future<Output = ()> {
        self.albedo.read_cpu_copy()
    }

    #[must_use]
    pub(crate) fn pixel_color(&self) -> Rc<Buffer> {
        self.pixel_color.gpu_copy()
    }

    #[must_use]
    pub(crate) fn object_id_at_gpu(&self) -> Rc<Buffer> {
        self.object_id.gpu_copy()
    }
    
    #[must_use]
    pub(crate) fn normal_at_gpu(&self) -> Rc<Buffer> {
        self.normal.gpu_copy()
    }

    #[must_use]
    pub(crate) fn albedo_gpu(&self) -> Rc<Buffer> {
        self.albedo.gpu_copy()
    }

    #[must_use]
    pub fn object_id_at_cpu(&self) -> &Vec<u32> {
        self.object_id.cpu_copy()
    }
    
    #[must_use]
    pub fn pixel_color_at_cpu(&self) -> &Vec<PodVector> {
        self.pixel_color.cpu_copy()
    }

    #[must_use]
    pub fn albedo_at_cpu(&self) -> &Vec<PodVector> {
        self.albedo.cpu_copy()
    }

    #[must_use]
    pub fn normal_at_cpu(&self) -> &Vec<PodVector> {
        self.normal.cpu_copy()
    }
    
    pub(crate) fn invalidate_object_id_and_normal(&mut self) {
        self.object_id.invalidate_cpu_copy();
        self.normal.invalidate_cpu_copy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use wgpu::wgt::PollType;
    use wgpu::CommandEncoderDescriptor;

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
        system_under_test.prepare_object_id_and_normal_copy_from_gpu(&mut encoder);
        context.queue().submit(Some(encoder.finish()));

        let gpu_to_cpu_copy = system_under_test.copy_all_from_gpu();
        context.device().poll(PollType::Wait).expect("failed to poll the device");
        pollster::block_on(gpu_to_cpu_copy);

        assert_eq!(system_under_test.object_id_at_cpu().len(), test_buffer_size().area() as usize);
        assert_eq!(system_under_test.normal_at_cpu().len(), test_buffer_size().area() as usize);
        assert_eq!(system_under_test.albedo_at_cpu().len(), test_buffer_size().area() as usize);
    }
}
