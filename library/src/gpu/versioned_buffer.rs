use std::rc::Rc;
use crate::gpu::resources::Resources;
use crate::scene::version::Version;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(super) struct BufferUpdateStatus {
    resized: bool,
    updated: bool,
}

impl BufferUpdateStatus {
    #[must_use]
    pub(crate) fn resized(&self) -> bool {
        self.resized
    }

    #[must_use]
    pub(crate) fn updated(&self) -> bool {
        self.updated
    }

    #[must_use]
    pub(crate) fn merge(&self, another: BufferUpdateStatus) -> Self {
        let resized = self.resized || another.resized;
        let updated = self.updated || another.updated;
        Self { resized, updated }
    }

    #[must_use] #[allow(dead_code)]
    pub(crate) fn new_resized(resized: bool) -> Self {
        Self { resized, updated: true }
    }

    #[must_use]
    pub(crate) fn new_updated(updated: bool) -> Self {
        Self { resized: false, updated }
    }
}

pub(super) struct VersionedBuffer {
    content_version: Version,
    backend: Rc<wgpu::Buffer>,
    elements_count: usize,
    label: &'static str,
}

impl VersionedBuffer {
    #[must_use]
    pub(super) fn new<Generator>(content_version: Version, resources: &Resources, label: &'static str, generate_data: Generator) -> Self
    where
        Generator: FnOnce() -> GpuReadySerializationBuffer,
    {
        let content = generate_data();
        let buffer = resources.create_storage_buffer_write_only(label, content.backend());
        let elements_count = content.total_slots_count();

        Self { content_version, backend: buffer, elements_count, label }
    }

    #[must_use]
    pub(super) fn version_diverges(&self, another: Version) -> bool {
        self.content_version != another
    }

    #[must_use]
    pub(super) fn try_update_and_resize<Generator>(&mut self, new_version: Version, resources: &Resources, queue: &wgpu::Queue, generate_data: Generator) -> BufferUpdateStatus
    where
        Generator: FnOnce() -> GpuReadySerializationBuffer,
    {
        if new_version == self.content_version {
            return BufferUpdateStatus { resized: false, updated: false };
        }

        self.content_version = new_version;

        let new_content = generate_data();
        self.elements_count = new_content.total_slots_count();

        if self.backend.size() >= new_content.backend().len() as u64 {
            queue.write_buffer(self.backend.as_ref(), 0, new_content.backend());
            return BufferUpdateStatus { resized: false, updated: true };
        }

        self.backend = resources.create_storage_buffer_write_only(self.label, new_content.backend());
        BufferUpdateStatus { resized: true, updated: true }
    }

    #[must_use]
    pub(super) fn backend(&self) -> &Rc<wgpu::Buffer> {
        &self.backend
    }
}

#[cfg(test)]
mod tests {
    use wgpu::TextureFormat;
    use crate::gpu::context::Context;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use super::*;

    #[must_use]
    fn make_test_content(slots_count: usize) -> GpuReadySerializationBuffer {
        GpuReadySerializationBuffer::make_filled(slots_count, 1, 7.0)
    }

    const SYSTEM_UNDER_TEST_INITIAL_SLOTS: usize = 2;
    const SYSTEM_UNDER_TEST_INITIAL_VERSION: Version = Version(0);
    const SYSTEM_UNDER_TEST_LABEL: &str = "test-buffer";

    #[must_use]
    fn make_system_under_test() -> (VersionedBuffer, Resources, Rc<Context>) {
        let context = create_headless_wgpu_context();
        let resources = Resources::new(context.clone(), TextureFormat::Rgba8Snorm);
        let generate_data = || make_test_content(SYSTEM_UNDER_TEST_INITIAL_SLOTS);

        let system_under_test = VersionedBuffer::new(SYSTEM_UNDER_TEST_INITIAL_VERSION, &resources, SYSTEM_UNDER_TEST_LABEL, generate_data);

        (system_under_test, resources, context.clone())
    }

    #[test]
    fn test_construction() {
        let (system_under_test, _, _) = make_system_under_test();

        let expected_content = make_test_content(SYSTEM_UNDER_TEST_INITIAL_SLOTS);
        assert_eq!(system_under_test.backend().size(), expected_content.backend().len() as u64);
    }

    #[test]
    fn test_try_update_and_resize_same_version() {
        let (mut system_under_test, resources, context) = make_system_under_test();
        let make_new_data = || make_test_content(1);

        let status = system_under_test.try_update_and_resize(
            SYSTEM_UNDER_TEST_INITIAL_VERSION,
            &resources,
            context.queue(),
            make_new_data);

        assert!(!status.resized());
    }

    #[test]
    fn test_try_update_and_resize_smaller_size() {
        let (mut system_under_test, resources, context) = make_system_under_test();
        let new_slots_count = SYSTEM_UNDER_TEST_INITIAL_SLOTS - 1;
        let make_new_data = || make_test_content(new_slots_count);

        let status = system_under_test.try_update_and_resize(
            SYSTEM_UNDER_TEST_INITIAL_VERSION + 1,
            &resources,
            context.queue(),
            make_new_data);

        assert!(!status.resized());
        let expected_content = make_new_data();
        assert!(system_under_test.backend().size() > expected_content.backend().len() as u64);
    }

    #[test]
    fn test_try_update_and_resize_bigger_size() {
        let (mut system_under_test, resources, context) = make_system_under_test();
        let new_slots_count = SYSTEM_UNDER_TEST_INITIAL_SLOTS + 1;
        let make_new_data = || make_test_content(new_slots_count);

        let status = system_under_test.try_update_and_resize(
            SYSTEM_UNDER_TEST_INITIAL_VERSION + 1,
            &resources,
            context.queue(),
            make_new_data);

        assert!(status.resized());
        let expected_content = make_new_data();
        assert_eq!(system_under_test.backend().size(), expected_content.backend().len() as u64);
    }
}