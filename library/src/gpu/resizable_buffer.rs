use crate::gpu::resources::Resources;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use std::rc::Rc;
use bytemuck::Pod;

pub(super) struct ResizableBuffer {
    backend: Rc<wgpu::Buffer>,
    label: &'static str,
}

#[derive(PartialEq, Debug)]
pub(super) enum ResizeStatus {
    Resized,
    SizeKept,
}

impl ResizableBuffer {
    #[must_use]
    fn new(resources: &Resources, label: &'static str, data: &[u8]) -> Self
    {
        Self {
            backend: resources.create_storage_buffer_write_only(label, data),
            label,
        }
    }
    
    #[must_use]
    pub(super) fn from_generator<Generator>(resources: &Resources, label: &'static str, generate_data: Generator) -> Self
    where
        Generator: FnOnce() -> GpuReadySerializationBuffer,
    {
        let content = generate_data();
        Self::new(resources, label, content.backend())
    }

    #[must_use]
    pub(super) fn from_slice<T: Pod>(resources: &Resources, label: &'static str, content: &[T]) -> Self {
        Self::new(resources, label, bytemuck::cast_slice(content))
    }

    fn update(&mut self, resources: &Resources, queue: &wgpu::Queue, data: &[u8]) -> ResizeStatus {
        if self.backend.size() >= data.len() as u64 {
            queue.write_buffer(self.backend.as_ref(), 0, data);
            ResizeStatus::SizeKept
        } else {
            self.backend = resources.create_storage_buffer_write_only(self.label, data);
            ResizeStatus::Resized   
        }
    }
    
    #[must_use]
    pub(super) fn update_with_generator<Generator>(&mut self, resources: &Resources, queue: &wgpu::Queue, generate_data: Generator) -> ResizeStatus
    where
        Generator: FnOnce() -> GpuReadySerializationBuffer,
    {
        let new_content = generate_data();
        self.update(resources, queue, new_content.backend())
    }

    #[must_use]
    pub(super) fn update_with_slice<T: Pod>(&mut self, resources: &Resources, queue: &wgpu::Queue, content: &[T]) -> ResizeStatus {
        self.update(resources, queue, bytemuck::cast_slice(content))
    }

    #[must_use]
    pub(super) fn backend(&self) -> &Rc<wgpu::Buffer> {
        &self.backend
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use crate::gpu::context::Context;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use super::*;

    #[must_use]
    fn make_test_content(slots_count: usize) -> GpuReadySerializationBuffer {
        GpuReadySerializationBuffer::make_filled(slots_count, 1, -3.0)
    }
    
    const SYSTEM_UNDER_TEST_INITIAL_SLOTS: usize = 2;
    
    #[must_use]
    fn make_system_under_test() -> (ResizableBuffer, Resources, Rc<Context>) {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context.clone());
        let generate_data = || make_test_content(SYSTEM_UNDER_TEST_INITIAL_SLOTS);

        let system_under_test = ResizableBuffer::from_generator(&resources, "test-buffer", generate_data);

        (system_under_test, resources, context)
    }

    #[rstest]
    #[case(-1, ResizeStatus::SizeKept)]
    #[case( 1, ResizeStatus::Resized )]
    fn test_update(#[case] slots_addition: i32, #[case] expected_status: ResizeStatus) {
        let (mut system_under_test, resources, context) = make_system_under_test();
        let new_slot_count = (SYSTEM_UNDER_TEST_INITIAL_SLOTS as i32 + slots_addition) as usize;
        let make_new_data = || make_test_content(new_slot_count);
        
        let actual_status = system_under_test.update_with_generator(&resources, context.queue(), make_new_data);
        
        assert_eq!(actual_status, expected_status);
    }
}
