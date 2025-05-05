use crate::Device;
use crate::sys::{OIDNBuffer, oidnGetBufferSize, oidnNewBuffer, oidnReadBuffer, oidnReleaseBuffer, oidnWriteBuffer, oidnWriteBufferAsync};
use std::mem;
use std::sync::Arc;

pub struct Buffer {
    pub(crate) buf: OIDNBuffer,
    pub(crate) f32_content_size: usize,
    pub(crate) device_arc: Arc<u8>,
}

impl Device {
    /// Creates a new buffer from a slice, returns None if buffer creation
    /// failed
    pub fn create_buffer_filled(&self, contents: &[f32]) -> Option<Buffer> {
        let byte_size = mem::size_of_val(contents);
        let buffer = unsafe {
            let buf = oidnNewBuffer(self.0, byte_size);
            if buf.is_null() {
                return None;
            } else {
                oidnWriteBuffer(buf, 0, byte_size, contents.as_ptr() as *const _);
                buf
            }
        };
        Some(Buffer {
            buf: buffer,
            f32_content_size: contents.len(),
            device_arc: self.1.clone(),
        })
    }

    /// Creates a new buffer of requested size
    /// failed
    pub fn create_buffer(&self, f32_count: usize) -> Option<Buffer> {
        assert!(f32_count > 0);
        let buffer = unsafe {
            let buf = oidnNewBuffer(self.0, f32_count * size_of::<f32>());
            if buf.is_null() {
                return None;
            } else {
                buf
            }
        };
        Some(Buffer {
            buf: buffer,
            f32_content_size: f32_count,
            device_arc: self.1.clone(),
        })
    }
    
    /// # Safety
    /// Raw buffer must not be invalid (e.g. destroyed, null ect.)
    ///
    /// Raw buffer must have been created by this device
    pub unsafe fn create_buffer_from_raw(&self, buffer: OIDNBuffer) -> Buffer {
        let size = unsafe { oidnGetBufferSize(buffer) } / mem::size_of::<f32>();
        Buffer {
            buf: buffer,
            f32_content_size: size,
            device_arc: self.1.clone(),
        }
    }

    pub(crate) fn same_device_as_buf(&self, buf: &Buffer) -> bool {
        self.1.as_ref() as *const _ as isize == buf.device_arc.as_ref() as *const _ as isize
    }
}

impl Buffer {
    /// Writes to the buffer, returns [None] if the sizes mismatch
    pub fn write(&self, contents: &[f32]) -> Option<()> {
        if self.f32_content_size < contents.len() {
            None
        } else {
            let byte_size = mem::size_of_val(contents);
            unsafe {
                oidnWriteBuffer(self.buf, 0, byte_size, contents.as_ptr() as *const _);
            }
            Some(())
        }
    }

    pub fn write_async(&self, contents: &[f32]) -> Option<()> {
        if self.f32_content_size < contents.len() {
            None
        } else {
            let byte_size = mem::size_of_val(contents);
            unsafe {
                oidnWriteBufferAsync(self.buf, 0, byte_size, contents.as_ptr() as *const _);
            }
            Some(())
        }
    }

    /// Reads from the buffer to the array, returns [None] if the sizes mismatch
    pub fn read_slice_into(&self, f32_count_to_read: usize, target: &mut [f32]) -> Option<()> {
        if self.f32_content_size < f32_count_to_read || f32_count_to_read > target.len() {
            None
        } else {
            let byte_size = f32_count_to_read * size_of::<f32>();
            unsafe {
                oidnReadBuffer(self.buf, 0, byte_size, target.as_ptr() as *mut _);
            }
            Some(())
        }
    }
    
    /// Reads from the buffer to the array, returns [None] if the sizes mismatch
    pub fn read_to_full_slice(&self, contents: &mut [f32]) -> Option<()> {
        if self.f32_content_size < contents.len() {
            None
        } else {
            let byte_size = mem::size_of_val(contents);
            unsafe {
                oidnReadBuffer(self.buf, 0, byte_size, contents.as_ptr() as *mut _);
            }
            Some(())
        }
    }

    /// Reads from the buffer
    pub fn read(&self) -> Vec<f32> {
        let contents = vec![0.0; self.f32_content_size];
        unsafe {
            oidnReadBuffer(
                self.buf,
                0,
                self.f32_content_size * mem::size_of::<f32>(),
                contents.as_ptr() as *mut _,
            );
        }
        contents
    }
    /// # Safety
    /// Raw buffer must not be made invalid (e.g. by destroying it)
    pub unsafe fn raw(&self) -> OIDNBuffer {
        self.buf
    }
    pub fn size(&self) -> usize {
        self.f32_content_size
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { oidnReleaseBuffer(self.buf) }
    }
}

unsafe impl Send for Buffer {}
