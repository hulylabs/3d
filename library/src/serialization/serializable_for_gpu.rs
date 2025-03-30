pub(crate) trait SerializableForGpu {
    const SERIALIZED_SIZE_FLOATS: usize;

    fn serialize_into(&self, buffer: &mut [f32]);
}
