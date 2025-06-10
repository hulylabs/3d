use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::duplex_layer::DuplexLayer;
use crate::serialization::pod_vector::PodVector;
use std::rc::Rc;
use wgpu::Buffer;
use crate::gpu::output::frame_buffer_layer::SupportUpdateFromCpu;

pub(crate) struct FrameBuffer {
    object_id: DuplexLayer<u32>,
    
    albedo: DuplexLayer<PodVector>,
    normal: DuplexLayer<PodVector>,

    noisy_pixel_color: DuplexLayer<PodVector>,
}

impl FrameBuffer {
    #[must_use]
    pub(crate) fn new(device: &wgpu::Device, frame_buffer_size: FrameBufferSize) -> Self {
        Self {
            object_id: DuplexLayer::new(device, frame_buffer_size, SupportUpdateFromCpu::No, "object id"),
            
            albedo: DuplexLayer::new(device, frame_buffer_size, SupportUpdateFromCpu::No, "albedo"),
            normal: DuplexLayer::new(device, frame_buffer_size, SupportUpdateFromCpu::No, "normal"),

            noisy_pixel_color: DuplexLayer::new(device, frame_buffer_size, SupportUpdateFromCpu::Yes, "noisy pixel color"),
        }
    }

    pub(crate) fn prepare_pixel_color_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.noisy_pixel_color.prepare_cpu_read(encoder);
    }
    
    pub(crate) fn prepare_all_aux_buffers_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.object_id.prepare_cpu_read(encoder);
        self.normal.prepare_cpu_read(encoder);
        self.albedo.prepare_cpu_read(encoder);
    }
    
    pub(crate) fn prepare_albedo_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.albedo.prepare_cpu_read(encoder);
    }

    pub(crate) fn prepare_object_id_copy_from_gpu(&self, encoder: &mut wgpu::CommandEncoder) {
        self.object_id.prepare_cpu_read(encoder);
    }
    
    pub(crate) fn copy_all_aux_buffers_from_gpu(&mut self) -> impl Future<Output = ()> {
        let object_id_read = self.object_id.read_cpu_copy();
        let normals_read = self.normal.read_cpu_copy();
        let albedo_read = self.albedo.read_cpu_copy();
        
        async move {
            futures::join!(object_id_read, normals_read, albedo_read);
        }
    }

    #[cfg(any(test, feature = "denoiser"))]
    pub(crate) fn copy_pixel_colors_from_gpu(&mut self) -> impl Future<Output = ()> {
        self.noisy_pixel_color.read_cpu_copy()
    }
    
    pub(crate) fn copy_albedo_from_gpu(&mut self) -> impl Future<Output = ()> {
        self.albedo.read_cpu_copy()
    }

    pub(crate) fn copy_object_id_from_gpu(&mut self) -> impl Future<Output = ()> {
        self.object_id.read_cpu_copy()
    }

    #[must_use]
    pub(crate) fn noisy_pixel_color(&self) -> Rc<Buffer> {
        self.noisy_pixel_color.gpu_copy()
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
    pub(crate) fn object_id_at_cpu(&self) -> &Vec<u32> {
        self.object_id.cpu_copy()
    }
    
    #[must_use] #[cfg(feature = "denoiser")]
    pub(crate) fn denoiser_input(&mut self) -> (&mut Vec<PodVector>, &Vec<PodVector>, &Vec<PodVector>) {
        (self.noisy_pixel_color.mutable_cpu_copy(), self.albedo.cpu_copy(), self.normal.cpu_copy())
    }

    #[cfg(test)]
    pub(crate) fn noisy_pixel_color_at_cpu(&self) -> &Vec<PodVector> {
        self.noisy_pixel_color.cpu_copy()
    }
    
    #[must_use]
    pub(crate) fn albedo_at_cpu_is_absent(&self) -> bool {
        self.albedo.cpu_copy().is_empty()
    }
    
    pub(crate) fn invalidate_cpu_copies(&mut self) {
        self.object_id.invalidate_cpu_copy();
        self.noisy_pixel_color.invalidate_cpu_copy();
        self.albedo.invalidate_cpu_copy();
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
        let system_under_test = test_aux_buffers_reading();
        
        assert_eq!(system_under_test.object_id_at_cpu().len(), test_buffer_size().area() as usize);
    }

    #[test] #[cfg(feature = "denoiser")]
    fn test_denoiser_input_acquiring() {
        let mut system_under_test = test_aux_buffers_reading();
        
        let (_, albedo_at_cpu, normal_at_cpu,) = system_under_test.denoiser_input();
        assert_eq!(albedo_at_cpu.len(), test_buffer_size().area() as usize);
        assert_eq!(normal_at_cpu.len(), test_buffer_size().area() as usize);
    }

    #[must_use]
    fn test_aux_buffers_reading() -> FrameBuffer {
        let context = create_headless_wgpu_context();

        let mut system_under_test = FrameBuffer::new(context.device(), test_buffer_size());

        let mut encoder = context.device().create_command_encoder(&CommandEncoderDescriptor { label: None });
        system_under_test.prepare_all_aux_buffers_copy_from_gpu(&mut encoder);
        context.queue().submit(Some(encoder.finish()));

        let gpu_to_cpu_copy = system_under_test.copy_all_aux_buffers_from_gpu();
        context.wait();
        pollster::block_on(gpu_to_cpu_copy);

        system_under_test
    }
}
