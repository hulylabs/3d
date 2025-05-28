use crate::serialization::gpu_ready_serialization_buffer::QUARTET_SIZE_BYTES;
use crate::serialization::single_quartet_writer::SingleQuartetWriter;

#[cfg(test)]
use crate::serialization::gpu_ready_serialization_buffer::QUARTET_ELEMENT_SIZE_BYTES;

pub(crate) struct SingleObjectWriter<'a> {
    storage: &'a mut Vec<u8>,
    write_pointer: usize,
    quartets_written: usize,
    quartets_per_object: usize,
}

impl<'a> SingleObjectWriter<'a> {
    #[must_use]
    pub(crate) fn fully_written(&self) -> bool {
        self.quartets_written == self.quartets_per_object
    }

    #[cfg(test)]
    fn write_element(&mut self, bytes: &[u8; QUARTET_ELEMENT_SIZE_BYTES]) {
        self.storage[self.write_pointer..self.write_pointer + QUARTET_ELEMENT_SIZE_BYTES].copy_from_slice(bytes);
        self.write_pointer += QUARTET_ELEMENT_SIZE_BYTES;
    }
    
    #[cfg(test)]
    pub(crate) fn write_quartet_f64(&mut self, x: f64, y: f64, z: f64, w: f64) {
        assert!(!self.fully_written());
        self.write_element(&(x as f32).to_ne_bytes());
        self.write_element(&(y as f32).to_ne_bytes());
        self.write_element(&(z as f32).to_ne_bytes());
        self.write_element(&(w as f32).to_ne_bytes());
        self.quartets_written += 1;
    }

    pub(crate) fn write_quartet<WritingCode>(&mut self, execute_writing: WritingCode)
    where
        WritingCode: FnOnce(&mut SingleQuartetWriter),
    {
        {
            let mut writer = SingleQuartetWriter::new(&mut self.storage, self.write_pointer);
            execute_writing(&mut writer);
        }
        self.write_pointer += QUARTET_SIZE_BYTES;
        self.quartets_written += 1;
    }

    #[must_use]
    pub(super) fn new(storage: &'a mut Vec<u8>, object_index: usize, quartets_per_object: usize) -> Self {
        let write_pointer = object_index * quartets_per_object * QUARTET_SIZE_BYTES;
        assert!(write_pointer + quartets_per_object * QUARTET_SIZE_BYTES <= storage.len());
        
        Self{
            storage,
            write_pointer,
            quartets_written: 0,
            quartets_per_object
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;

    #[test]
    fn test_construction() {
        let mut storage: Vec<u8> = vec![0u8; QUARTET_SIZE_BYTES];
        let writer = SingleObjectWriter::new(&mut storage, 0, 1);

        assert!(!writer.fully_written());
    }

    #[test]
    #[should_panic]
    fn test_construction_with_bad_object_index() {
        let mut storage: Vec<u8> = vec![0u8; QUARTET_SIZE_BYTES];
        let _ = SingleObjectWriter::new(&mut storage, 2, 1);
    }

    #[test]
    #[should_panic]
    fn test_construction_with_too_many_bytes_per_object() {
        let mut storage: Vec<u8> = vec![0u8; QUARTET_SIZE_BYTES];
        let _ = SingleObjectWriter::new(&mut storage, 0, 2);
    }
    #[test]
    fn test_fully_written_after_all_quartets() {
        let mut storage = vec![0u8; QUARTET_SIZE_BYTES];
        let mut writer = SingleObjectWriter::new(&mut storage, 0, 1);

        writer.write_quartet_f64(1.0, 2.0, 3.0, 4.0);

        assert!(writer.fully_written());
    }

    #[test]
    fn test_write_quartet_f64_single() {
        let mut storage = vec![0u8; QUARTET_SIZE_BYTES];
        let mut writer = SingleObjectWriter::new(&mut storage, 0, 1);

        let x: f32 = 1.5;
        let y: f32 = 2.5;
        let z: f32 = 3.5;
        let w: f32 = 4.5;
        writer.write_quartet_f64(x as f64, y as f64, z as f64, w as f64);
        
        assert_eq!(&storage[0..4]  , &x.to_ne_bytes());
        assert_eq!(&storage[4..8]  , &y.to_ne_bytes());
        assert_eq!(&storage[8..12] , &z.to_ne_bytes());
        assert_eq!(&storage[12..16], &w.to_ne_bytes());
    }

    #[test]
    fn test_write_quartet_with_closure() {
        let mut storage = vec![0u8; QUARTET_SIZE_BYTES];
        let mut writer = SingleObjectWriter::new(&mut storage, 0, 1);

        let first: f32 = 10.5;
        let second: i32 = -20;
        let third: u32 = 30;
        
        writer.write_quartet(|quartet_writer| {
            quartet_writer.write_float_32(first);
            quartet_writer.write_signed(second);
            quartet_writer.write_unsigned(third);
        });
        
        assert_eq!(&storage[0..4] , &first.to_ne_bytes());
        assert_eq!(&storage[4..8] , &second.to_ne_bytes());
        assert_eq!(&storage[8..12], &third.to_ne_bytes());
        assert_eq!(&storage[12..],  &DEFAULT_PAD_VALUE.to_ne_bytes());
    }
}