use crate::serialization::gpu_ready_serialization_buffer::{DEFAULT_PAD_VALUE, ELEMENTS_IN_QUARTET, QUARTET_ELEMENT_SIZE_BYTES, QUARTET_SIZE_BYTES};

pub(crate) struct SingleQuartetWriter<'a> {
    storage: &'a mut Vec<u8>,
    write_pointer: usize,
    elements_written: usize,
}

impl Drop for SingleQuartetWriter<'_> {
    fn drop(&mut self) {
        while self.elements_written < ELEMENTS_IN_QUARTET {
            self.write_float_32(DEFAULT_PAD_VALUE);
        }
    }
}

impl<'a> SingleQuartetWriter<'a> {
    #[must_use]
    pub(super) fn new(storage: &'a mut Vec<u8>, write_pointer: usize) -> Self {
        assert!(write_pointer + QUARTET_SIZE_BYTES <= storage.len());
        Self {
            storage,
            write_pointer,
            elements_written: 0,
        }
    }

    fn write_element(&mut self, bytes: &[u8; QUARTET_ELEMENT_SIZE_BYTES]) {
        self.storage[self.write_pointer..self.write_pointer + QUARTET_ELEMENT_SIZE_BYTES].copy_from_slice(bytes);
        self.elements_written += 1;
        self.write_pointer += QUARTET_ELEMENT_SIZE_BYTES;
    }

    pub(crate) fn write_unsigned(&mut self, value: u32) -> &mut Self {
        assert!(self.elements_written < ELEMENTS_IN_QUARTET);
        self.write_element(&value.to_ne_bytes());
        self
    }
    
    pub(crate) fn write_signed(&mut self, value: i32) -> &mut Self {
        assert!(self.elements_written < ELEMENTS_IN_QUARTET);
        self.write_element(&value.to_ne_bytes());
        self
    }

    pub(crate) fn write_float_32(&mut self, value: f32) -> &mut Self {
        assert!(self.elements_written < ELEMENTS_IN_QUARTET);
        self.write_element(&value.to_ne_bytes());
        self
    }
    
    pub(crate) fn write_float_64(&mut self, value: f64) -> &mut Self {
        assert!(self.elements_written < ELEMENTS_IN_QUARTET);
        self.write_float_32(value as f32)
    }
}