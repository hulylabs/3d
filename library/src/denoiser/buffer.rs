use crate::denoiser::sys::{oidnNewBuffer, oidnReadBuffer, oidnReleaseBuffer, oidnWriteBufferAsync, OIDNBuffer};
use std::sync::Arc;
use crate::denoiser::device::Device;

pub struct Buffer {
    pub(crate) buffer: OIDNBuffer,
    pub(crate) f32_content_size: usize,
    pub(crate) device_arc: Arc<u8>,
}

impl Device {
    /// Creates a new buffer of requested size, returns None if buffer creation
    /// failed
    #[must_use]
    pub fn create_buffer(&self, f32_count: usize) -> Option<Buffer> {
        assert!(f32_count > 0);
        let buffer = unsafe {
            let buffer = oidnNewBuffer(self.0, f32_count * size_of::<f32>());
            if buffer.is_null() {
                return None;
            } else {
                buffer
            }
        };
        Some(Buffer {
            buffer,
            f32_content_size: f32_count,
            device_arc: self.1.clone(),
        })
    }

    #[must_use]
    pub(crate) fn same_device_as(&self, buffer: &Buffer) -> bool {
        self.1.as_ref() as *const _ as isize == buffer.device_arc.as_ref() as *const _ as isize
    }
}

#[cfg(test)]
impl Device {
    /// # Safety
    /// Raw buffer must not be invalid (e.g. destroyed, null ect.)
    ///
    /// Raw buffer must have been created by this device
    #[must_use]
    pub unsafe fn create_buffer_from_raw(&self, buffer: OIDNBuffer) -> Buffer {
        let size_bytes = unsafe { crate::denoiser::sys::oidnGetBufferSize(buffer) };
        assert_eq!(size_bytes % size_of::<f32>(), 0);

        Buffer {
            buffer,
            f32_content_size: size_bytes / size_of::<f32>(),
            device_arc: self.1.clone(),
        }
    }
}

impl Buffer {
    pub fn write_async(&self, contents: &[f32]) -> Option<()> {
        if self.f32_content_size < contents.len() {
            None
        } else {
            let byte_size = size_of_val(contents);
            unsafe {
                oidnWriteBufferAsync(self.buffer, 0, byte_size, contents.as_ptr() as *const _);
            }
            Some(())
        }
    }

    /// Reads from the buffer to the array, returns [None] if the sizes mismatch
    pub fn read_slice_into(&self, f32_count_to_read: usize, target: &mut [f32]) -> Option<()> {
        assert!(f32_count_to_read > 0);
        if self.f32_content_size < f32_count_to_read || f32_count_to_read > target.len() {
            None
        } else {
            let byte_size = f32_count_to_read * size_of::<f32>();
            unsafe {
                oidnReadBuffer(self.buffer, 0, byte_size, target.as_ptr() as *mut _);
            }
            Some(())
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { oidnReleaseBuffer(self.buffer) }
    }
}

unsafe impl Send for Buffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_read_write() {
        let device = Device::new();
        let buffer = match device.create_buffer(1) {
            Some(buffer) => {
                buffer.write_async(&[0.0]);
                device.sync();
                buffer
            },
            // resources failing to be created is not the fault of this library
            None => {
                eprintln!("Test skipped due to buffer creation failing");
                return;
            }
        };

        let test_value_to_write = 7.0;

        buffer.write_async(&[test_value_to_write]).unwrap();
        device.sync();

        let mut slice = vec![0.0];
        buffer.read_slice_into(slice.len(), &mut slice).unwrap();
        assert_eq!(slice, vec![test_value_to_write]);

        if let Err((err, str)) = device.get_error() {
            panic!("test failed with {err:?}: {str}")
        }
    }

    #[cfg(test)]
    #[test]
    fn buffer_import_read_write() {
        let device = Device::new();
        let raw_buffer = unsafe { oidnNewBuffer(device.raw(), size_of::<f32>()) };
        if raw_buffer.is_null() {
            eprintln!("Test skipped due to buffer creation failing");
            return;
        }

        let test_value_to_write = 13.0;

        let buffer = unsafe { device.create_buffer_from_raw(raw_buffer) };
        buffer.write_async(&[test_value_to_write]).unwrap();
        device.sync();

        let mut slice = vec![0.0];
        buffer.read_slice_into(slice.len(), &mut slice).unwrap();
        assert_eq!(slice, vec![test_value_to_write]);

        if let Err((err, str)) = device.get_error() {
            panic!("test failed with {err:?}: {str}")
        }
    }

    #[test]
    fn test_smaller_range_read_write() {
        let device = Device::new();
        let system_under_test = device.create_buffer(7).unwrap();
        let content_to_write = [3.0, 5.0, 7.0];
        system_under_test.write_async(&content_to_write);
        device.sync();

        for numbers_to_read in 1..content_to_write.len() {
            let mut slice = vec![0.0; numbers_to_read];
            system_under_test.read_slice_into(slice.len(), &mut slice).unwrap();
            assert_eq!(slice, content_to_write[0..numbers_to_read]);   
        }
    }
}