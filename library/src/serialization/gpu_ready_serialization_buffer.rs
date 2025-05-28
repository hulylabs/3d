use std::vec;
use crate::serialization::single_object_writer::SingleObjectWriter;
use crate::serialization::single_quartet_writer::SingleQuartetWriter;

pub(crate) const ELEMENTS_IN_QUARTET: usize = 4;
pub(super) const QUARTET_ELEMENT_SIZE_BYTES: usize = size_of::<f32>();
pub(super) const QUARTET_SIZE_BYTES: usize = QUARTET_ELEMENT_SIZE_BYTES * ELEMENTS_IN_QUARTET;

pub(crate) const DEFAULT_PAD_VALUE: f32 = -1.0;

pub(crate) struct GpuReadySerializationBuffer {
    backend: Vec<u8>,
    write_pointer: usize,
    quartets_per_object: usize,
}

impl GpuReadySerializationBuffer {
    #[must_use]
    pub(crate) fn new(objects_count: usize, quartets_per_object: usize) -> Self {
        assert!(quartets_per_object > 0);
        Self {
            backend: vec![0; Self::backend_size_bytes(objects_count, quartets_per_object)],
            write_pointer: 0,
            quartets_per_object,
        }
    }

    #[must_use]
    pub(crate) fn total_slots_count(&self) -> usize {
        self.backend.len() / (self.quartets_per_object * QUARTET_SIZE_BYTES)
    }

    #[must_use]
    pub(crate) fn make_filled(objects_count: usize, quartets_per_object: usize, filler: f32) -> Self {
        assert!(quartets_per_object > 0);

        let mut result = Self::new(objects_count, quartets_per_object);
        loop {
            if result.fully_written() {
                break;
            }
            result.write_quartet_f32(filler, filler, filler, filler);
        }

        result
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.backend.is_empty()
    }

    #[must_use]
    fn backend_size_bytes(objects_count_capacity: usize, quartets_per_object: usize) -> usize {
        objects_count_capacity * quartets_per_object * QUARTET_SIZE_BYTES
    }

    #[must_use]
    fn bytes_per_object(&self) -> usize {
        self.quartets_per_object * QUARTET_SIZE_BYTES
    }

    #[must_use]
    pub(crate) fn free_quartets_of_current_object(&self) -> usize {
        let object_start = self.write_pointer - self.write_pointer % self.bytes_per_object();
        let object_end = object_start + self.bytes_per_object();
        (object_end - self.write_pointer) / QUARTET_SIZE_BYTES
    }

    #[must_use]
    pub(crate) fn object_fully_written(&self) -> bool {
        0 < self.write_pointer && (0 == self.write_pointer % self.bytes_per_object())
    }

    #[must_use]
    pub(crate) fn fully_written(&self) -> bool {
        self.write_pointer == self.backend.len()
    }

    #[must_use]
    pub(crate) fn has_free_slot(&self) -> bool {
        ! self.fully_written()
    }

    #[must_use]
    pub(crate) fn backend(&self) -> &Vec<u8> {
        assert!(self.fully_written(), "buffer has not been filled");
        &self.backend
    }

    pub(crate) fn write_object<WritingCode>(&mut self, element_index: usize, execute_writing: WritingCode)
    where
        WritingCode: FnOnce(&mut SingleObjectWriter)
    {
        assert!(self.fully_written(), "buffer has not been filled");

        let offset = element_index * self.bytes_per_object();
        assert!(offset + self.bytes_per_object() <= self.backend.len());

        let mut writer = SingleObjectWriter::new(&mut self.backend, element_index, self.quartets_per_object);
        execute_writing(&mut writer);

        assert!(writer.fully_written());
    }

    pub(crate) fn write_quartet_f64(&mut self, x: f64, y: f64, z: f64, w: f64) {
        self.write_quartet_f32(x as f32, y as f32, z as f32, w as f32);
    }

    pub(crate) fn write_padded_quartet_f64(&mut self, x: f64, y: f64, z: f64) {
        self.write_quartet_f64(x, y, z, DEFAULT_PAD_VALUE as f64);
    }

    pub(crate) fn write_quartet_f32(&mut self, x: f32, y: f32, z: f32, w: f32) {
        self.write_quartet(|writer| {
            writer.write_float_32(x).write_float_32(y).write_float_32(z).write_float_32(w);
        });
    }

    pub(crate) fn write_quartet_u32(&mut self, x: u32, y: u32, z: u32, w: u32) {
        self.write_quartet(|writer| {
            writer.write_unsigned(x).write_unsigned(y).write_unsigned(z).write_unsigned(w);
        });
    }

    pub(crate) fn write_padded_quartet_f32(&mut self, x: f32, y: f32, z: f32) {
        self.write_quartet_f32(x, y, z, DEFAULT_PAD_VALUE);
    }

    pub(crate) fn write_quartet<WritingCode>(&mut self, execute_writing: WritingCode)
    where
        WritingCode: FnOnce(&mut SingleQuartetWriter),
    {
        {
            let mut writer = SingleQuartetWriter::new(&mut self.backend, self.write_pointer);
            execute_writing(&mut writer);
        }
        self.write_pointer += QUARTET_SIZE_BYTES;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_initialization() {
        let system_under_test = GpuReadySerializationBuffer::new(5, 3);

        assert!(!system_under_test.object_fully_written());
        assert!(!system_under_test.fully_written());
    }

    #[test]
    #[should_panic]
    fn test_buffer_initialization_fails_with_zero_quartets() {
        let _ = GpuReadySerializationBuffer::new(1, 0);
    }

    #[test]
    fn test_write_quartet() {
        let expected_quartets_per_object = 2;
        let mut system_under_test = GpuReadySerializationBuffer::new(1, expected_quartets_per_object);

        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        assert!(!system_under_test.fully_written());
        assert!(!system_under_test.object_fully_written());

        system_under_test.write_quartet_f32(5.0, 6.0, 7.0, 8.0);
        assert!(system_under_test.object_fully_written());
        assert!(system_under_test.fully_written());

        let backend = system_under_test.backend();

        let mut offset = 0;
        for i in 0..(expected_quartets_per_object * ELEMENTS_IN_QUARTET) {
            assert_eq!(f32::from_ne_bytes(backend[offset..offset+4].try_into().unwrap()), (i+1) as f32);
            offset += 4;
        }
    }

    #[test]
    fn test_write_using_closure() {
        let mut system_under_test = GpuReadySerializationBuffer::new(1, 1);

        system_under_test.write_quartet(|writer| {
            writer
                .write_float_32(10.0)
                .write_signed(-20)
                .write_unsigned(30)
                .write_float_32(40.0)
            ;
        });

        assert!(system_under_test.object_fully_written());
        assert!(system_under_test.fully_written());

        let backend = system_under_test.backend();

        let mut offset = 0;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), 10.0);
        offset += 4;
        assert_eq!(i32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), -20);
        offset += 4;
        assert_eq!(u32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), 30);
        offset += 4;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), 40.0);
    }

    #[test]
    #[should_panic]
    fn test_backend_access_before_fully_written() {
        let mut system_under_test = GpuReadySerializationBuffer::new(2, 1);
        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        let _backend = system_under_test.backend();
    }

    #[test]
    fn test_single_quartet_writer_auto_padding() {
        let mut system_under_test = GpuReadySerializationBuffer::new(1, 1);

        system_under_test.write_quartet(|writer| {
            writer.write_float_32(1.0).write_float_32(2.0);
        });

        assert!(system_under_test.fully_written());

        let backend = system_under_test.backend();

        let mut offset = 0;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), 1.0);
        offset += 4;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), 2.0);
        offset += 4;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), DEFAULT_PAD_VALUE);
        offset += 4;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap()), DEFAULT_PAD_VALUE);
    }

    #[test]
    fn test_multiple_objects() {
        let mut system_under_test = GpuReadySerializationBuffer::new(2, 2);

        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        system_under_test.write_quartet_f32(5.0, 6.0, 7.0, 8.0);
        assert!(system_under_test.object_fully_written());
        assert!(!system_under_test.fully_written());

        system_under_test.write_quartet_f32(9.0, 10.0, 11.0, 12.0);
        system_under_test.write_quartet_f32(13.0, 14.0, 15.0, 16.0);
        assert!(system_under_test.object_fully_written());
        assert!(system_under_test.fully_written());

        let backend = system_under_test.backend();
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];

        for (i, expected) in values.iter().enumerate() {
            let offset = i * size_of::<f32>();
            let actual = f32::from_ne_bytes(backend[offset..offset+QUARTET_ELEMENT_SIZE_BYTES].try_into().unwrap());
            assert_eq!(actual, *expected, "Mismatch at index {}", i);
        }
    }

    #[test]
    fn test_mixed_integer_and_float_writing() {
        let mut system_under_test = GpuReadySerializationBuffer::new(1, 1);

        system_under_test.write_quartet(|writer| {
            writer
                .write_unsigned(42)
                .write_float_32(3.14)
                .write_unsigned(7)
                .write_float_32(2.71);
        });

        assert!(system_under_test.fully_written());

        let backend = system_under_test.backend();

        let mut offset = 0;
        assert_eq!(i32::from_ne_bytes(backend[offset..offset+4].try_into().unwrap()), 42);
        offset += 4;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+4].try_into().unwrap()), 3.14);
        offset += 4;
        assert_eq!(i32::from_ne_bytes(backend[offset..offset+4].try_into().unwrap()), 7);
        offset += 4;
        assert_eq!(f32::from_ne_bytes(backend[offset..offset+4].try_into().unwrap()), 2.71);
    }

    #[test]
    #[should_panic]
    fn test_write_more_than_four_elements() {
        let mut system_under_test = GpuReadySerializationBuffer::new(1, 1);

        system_under_test.write_quartet(|writer| {
            writer
                .write_float_32(1.0)
                .write_float_32(2.0)
                .write_float_32(3.0)
                .write_float_32(4.0)
                .write_float_32(5.0);
        });
    }

    #[test]
    #[should_panic]
    fn test_write_beyond_capacity() {
        let mut system_under_test = GpuReadySerializationBuffer::new(1, 1);
        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        system_under_test.write_quartet_f32(5.0, 6.0, 7.0, 8.0);
    }

    #[test]
    fn test_free_quartets_of_current_object() {
        let quartets_per_object = 3;

        let system_under_test = GpuReadySerializationBuffer::new(1, quartets_per_object);
        assert_eq!(system_under_test.free_quartets_of_current_object(), quartets_per_object);

        let mut system_under_test = GpuReadySerializationBuffer::new(2, quartets_per_object);

        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        assert_eq!(system_under_test.free_quartets_of_current_object(), 2);

        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        assert_eq!(system_under_test.free_quartets_of_current_object(), 1);

        system_under_test.write_quartet_f32(1.0, 2.0, 3.0, 4.0);
        assert_eq!(system_under_test.free_quartets_of_current_object(), 3);
    }
}