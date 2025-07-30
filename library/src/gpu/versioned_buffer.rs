use std::rc::Rc;
use bytemuck::Pod;
use crate::utils::version::Version;
use crate::gpu::resizable_buffer::{ResizableBuffer, ResizeStatus};
use crate::gpu::resources::Resources;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(super) struct VersionedBuffer {
    content_version: Version,
    backend: ResizableBuffer,
}

pub(super) struct BufferUpdateStatus {
    resized: bool,
    updated: bool,
}

impl BufferUpdateStatus {
    #[must_use]
    pub(super) fn resized(&self) -> bool {
        self.resized
    }

    #[must_use]
    pub(super) fn updated(&self) -> bool {
        self.updated
    }

    #[must_use]
    pub(super) fn merge(&self, another: BufferUpdateStatus) -> Self {
        let resized = self.resized || another.resized;
        let updated = self.updated || another.updated;
        Self { resized, updated }
    }

    #[must_use]
    pub(super) fn new_updated(updated: bool) -> Self {
        Self { resized: false, updated }
    }

    #[must_use]
    pub(super) fn new(resized: bool, updated: bool) -> Self {
        Self { resized, updated }
    }
}

impl VersionedBuffer {
    #[must_use]
    pub(super) fn from_generator<Generator>(content_version: Version, resources: &Resources, label: &'static str, generate_data: Generator) -> Self
    where
        Generator: FnOnce() -> GpuReadySerializationBuffer,
    {
        Self { content_version, backend: ResizableBuffer::from_generator(resources, label, generate_data) }
    }

    #[must_use]
    pub(super) fn from_slice<T: Pod>(content_version: Version, resources: &Resources, label: &'static str, slice: &[T]) -> Self {
        Self { content_version, backend: ResizableBuffer::from_slice(resources, label, slice) }
    }

    #[must_use]
    pub(super) fn version_diverges(&self, another: Version) -> bool {
        self.content_version != another
    }

    #[must_use]
    pub(super) fn try_update_with_generator<Generator>(&mut self, new_version: Version, resources: &Resources, queue: &wgpu::Queue, generate_data: Generator) -> BufferUpdateStatus
    where
        Generator: FnOnce() -> GpuReadySerializationBuffer,
    {
        if new_version == self.content_version {
            return BufferUpdateStatus { resized: false, updated: false };
        }

        self.content_version = new_version;

        let resized = self.backend.update_with_generator(resources, queue, generate_data);
        BufferUpdateStatus { resized: ResizeStatus::Resized == resized, updated: true }
    }

    #[must_use]
    pub(super) fn try_update_with_slice<T: Pod>(&mut self, new_version: Version, resources: &Resources, queue: &wgpu::Queue, slice: &[T]) -> BufferUpdateStatus {
        if new_version == self.content_version {
            return BufferUpdateStatus { resized: false, updated: false };
        }

        self.content_version = new_version;

        let resized = self.backend.update_with_slice(resources, queue, slice);
        BufferUpdateStatus { resized: ResizeStatus::Resized == resized, updated: true }
    }

    #[must_use]
    pub(super) fn backend(&self) -> &Rc<wgpu::Buffer> {
        self.backend.backend()
    }
}

#[cfg(test)]
mod tests {
    use test_context::{test_context, TestContext};
    use crate::gpu::context::Context;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use super::*;

    impl BufferUpdateStatus {
        #[must_use]
        pub(crate) fn new_resized(resized: bool) -> Self {
            Self { resized, updated: true }
        }
    }
    
    #[must_use]
    fn make_test_content(slots_count: usize) -> GpuReadySerializationBuffer {
        GpuReadySerializationBuffer::make_filled(slots_count, 1, 7.0)
    }

    const SYSTEM_UNDER_TEST_INITIAL_SLOTS: usize = 2;
    const SYSTEM_UNDER_TEST_INITIAL_VERSION: Version = Version(0);

    struct Fixture {
        system_under_test: VersionedBuffer,
        resources: Resources,
        context: Rc<Context>,
    }

    impl TestContext for Fixture {
        fn setup() -> Fixture {
            let context = create_headless_wgpu_context();
            let resources = Resources::new(context.clone());
            let generate_data = || make_test_content(SYSTEM_UNDER_TEST_INITIAL_SLOTS);

            let system_under_test = VersionedBuffer::from_generator(SYSTEM_UNDER_TEST_INITIAL_VERSION, &resources, "test-buffer", generate_data);

            Fixture { system_under_test, resources, context }
        }

        fn teardown(self) {
        }
    }

    #[test_context(Fixture)]
    #[test]
    fn test_construction(fixture: &mut Fixture) {
        let expected_content = make_test_content(SYSTEM_UNDER_TEST_INITIAL_SLOTS);
        assert_eq!(fixture.system_under_test.backend().size(), expected_content.backend().len() as u64);
    }

    #[test_context(Fixture)]
    #[test]
    fn test_try_update_and_resize_same_version(fixture: &mut Fixture) {
        let make_new_data = || make_test_content(1);

        let status = fixture.system_under_test.try_update_with_generator(
            SYSTEM_UNDER_TEST_INITIAL_VERSION,
            &fixture.resources,
            fixture.context.queue(),
            make_new_data);

        assert!(!status.resized());
    }

    #[test_context(Fixture)]
    #[test]
    fn test_try_update_and_resize_smaller_size(fixture: &mut Fixture) {
        let new_slots_count = SYSTEM_UNDER_TEST_INITIAL_SLOTS - 1;
        let make_new_data = || make_test_content(new_slots_count);

        let status = fixture.system_under_test.try_update_with_generator(
            SYSTEM_UNDER_TEST_INITIAL_VERSION + 1,
            &fixture.resources,
            fixture.context.queue(),
            make_new_data);

        assert!(!status.resized());
        let expected_content = make_new_data();
        assert!(fixture.system_under_test.backend().size() > expected_content.backend().len() as u64);
    }

    #[test_context(Fixture)]
    #[test]
    fn test_try_update_and_resize_bigger_size(fixture: &mut Fixture) {
        let new_slots_count = SYSTEM_UNDER_TEST_INITIAL_SLOTS + 1;
        let make_new_data = || make_test_content(new_slots_count);

        let status = fixture.system_under_test.try_update_with_generator(
            SYSTEM_UNDER_TEST_INITIAL_VERSION + 1,
            &fixture.resources,
            fixture.context.queue(),
            make_new_data);

        assert!(status.resized());
        let expected_content = make_new_data();
        assert_eq!(fixture.system_under_test.backend().size(), expected_content.backend().len() as u64);
    }
}