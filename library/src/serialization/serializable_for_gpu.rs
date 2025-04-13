use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(crate) trait SerializableForGpu {
    const SERIALIZED_QUARTET_COUNT: usize;

    fn serialize_into(&self, buffer: &mut GpuReadySerializationBuffer);
}
