use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(crate) trait GpuSerializationSize {
    const SERIALIZED_QUARTET_COUNT: usize;
}

pub(crate) trait GpuSerializable {
    fn serialize_into(&self, buffer: &mut GpuReadySerializationBuffer);
}

#[must_use]
pub(crate) fn serialize_batch<T>(objects: &Vec<T>) -> GpuReadySerializationBuffer
where
    T: GpuSerializable + GpuSerializationSize,
{
    let mut buffer = GpuReadySerializationBuffer::new(objects.len(), T::SERIALIZED_QUARTET_COUNT);
    for object in objects {
        object.serialize_into(&mut buffer)
    }
    buffer
}
