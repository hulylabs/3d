use crate::denoiser::error::Error;
use crate::denoiser::sys::OIDNDevice;
use crate::denoiser::sys::*;
use std::sync::Arc;
use std::{ffi::CStr, os::raw::c_char, ptr};

/// An Open Image Denoise device (e.g. a CPU).
///
/// Open Image Denoise supports a device concept, which allows different
/// components of the application to use the API without interfering with each
/// other.
///
/// While all API calls on a device are thread-safe, they may be serialized.
/// Therefor, it is recommended to call from the same thread.
pub struct Device(pub(crate) OIDNDevice, pub(crate) Arc<u8>);

#[cfg(test)]
impl Device {
    pub(crate) fn sync(&self) {
        unsafe {
            oidnSyncDevice(self.0);
        }
    }

    /// # Safety
    /// Raw device must not be made invalid (e.g. by destroying it).
    pub(crate) unsafe fn raw(&self) -> OIDNDevice {
        self.0
    }
}

impl Device {
    /// Create a device using the fastest device available to run denoising
    #[must_use]
    pub fn new() -> Self {
        Self::create(OIDNDeviceType_OIDN_DEVICE_TYPE_DEFAULT)
    }

    #[must_use]
    fn create(device_type: OIDNDeviceType) -> Self {
        let handle = get_handle(device_type);
        unsafe {
            oidnCommitDevice(handle);
        }
        Self(handle, Arc::new(0))
    }
    
    pub fn get_error(&self) -> Result<(), (Error, String)> {
        let mut err_msg = ptr::null();
        let err = unsafe { oidnGetDeviceError(self.0, &mut err_msg as *mut *const c_char) };
        if OIDNError_OIDN_ERROR_NONE == err {
            Ok(())
        } else {
            let msg = unsafe { CStr::from_ptr(err_msg).to_string_lossy().to_string() };
            Err(((err as u32).try_into().unwrap(), msg))
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            oidnReleaseDevice(self.0);
        }
    }
}

impl Default for Device {
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for Device {}

fn get_handle(device_type: u32) -> *mut OIDNDeviceImpl {
    unsafe { oidnNewDevice(device_type) }
}
