use num_enum::TryFromPrimitive;
use crate::denoiser::sys;

#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, TryFromPrimitive)]
pub(super) enum Error {
    None = sys::OIDNError_OIDN_ERROR_NONE,
    Unknown = sys::OIDNError_OIDN_ERROR_UNKNOWN,
    InvalidArgument = sys::OIDNError_OIDN_ERROR_INVALID_ARGUMENT,
    InvalidOperation = sys::OIDNError_OIDN_ERROR_INVALID_OPERATION,
    OutOfMemory = sys::OIDNError_OIDN_ERROR_OUT_OF_MEMORY,
    UnsupportedFormat = sys::OIDNError_OIDN_ERROR_UNSUPPORTED_HARDWARE,
    Canceled = sys::OIDNError_OIDN_ERROR_CANCELLED,
    InvalidImageDimensions,
}