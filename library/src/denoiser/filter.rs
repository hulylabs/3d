use crate::denoiser::{buffer::Buffer, device::Device, sys::*};
use std::rc::Rc;
use num_enum::TryFromPrimitive;
use crate::denoiser::error::Error;

#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, TryFromPrimitive, Default)]
pub(crate) enum Quality {
    #[default]
    Default = OIDNQuality_OIDN_QUALITY_DEFAULT,
    Balanced = OIDNQuality_OIDN_QUALITY_BALANCED,
    High = OIDNQuality_OIDN_QUALITY_HIGH,
    Fast = OIDNQuality_OIDN_QUALITY_FAST,
}

impl Quality {
    pub fn as_raw_oidn_quality(&self) -> OIDNQuality {
        match self {
            Quality::Default => OIDNQuality_OIDN_QUALITY_DEFAULT,
            Quality::Balanced => OIDNQuality_OIDN_QUALITY_BALANCED,
            Quality::High => OIDNQuality_OIDN_QUALITY_HIGH,
            Quality::Fast => OIDNQuality_OIDN_QUALITY_FAST,
        }
    }
}

/// A generic ray tracing denoising filter for denoising
/// images produces with Monte Carlo ray tracing methods
/// such as path tracing.
pub struct RayTracing {
    handle: OIDNFilter,
    device: Rc<Device>,
    albedo: Option<Rc<Buffer>>,
    normal: Option<Rc<Buffer>>,
    hdr: bool,
    input_scale: Option<f32>,
    srgb: bool,
    clean_aux: bool,
    img_dims: (usize, usize, usize),
    filter_quality: OIDNQuality,
    image_channel_per_pixel: usize,
}

impl RayTracing {
    #[must_use]
    pub fn new(device: Rc<Device>, image_channel_per_pixel: usize,) -> Self {
        assert!(image_channel_per_pixel > 0);
        unsafe {
            oidnRetainDevice(device.0);
        }
        let filter = unsafe { oidnNewFilter(device.0, b"RT\0" as *const _ as _) };
        Self {
            handle: filter,
            device,
            albedo: None,
            normal: None,
            hdr: false,
            input_scale: None,
            srgb: false,
            clean_aux: false,
            img_dims: (0, 0, 0),
            filter_quality: 0,
            image_channel_per_pixel,
        }
    }

    /// Sets the quality of the output, the default is high.
    ///
    /// Balanced lowers the precision, if possible, however
    /// some devices will not support this and so
    /// the result (and performance) will stay the same as high.
    /// Balanced is recommended for realtime usages.
    pub fn filter_quality(&mut self, quality: Quality) -> &mut Self {
        self.filter_quality = quality.as_raw_oidn_quality();
        self
    }
    
    /// Set input auxiliary buffer containing the albedo and normals.
    ///
    /// Albedo buffer must have three channels per pixel with values in [0, 1].
    /// Normal must contain the shading normal as three channels per pixel
    /// *world-space* or *view-space* vectors with arbitrary length, values
    /// in `[-1, 1]`.
    ///
    /// This function is the same as [RayTracing::albedo_normal] but takes
    /// buffers instead
    ///
    /// Returns [None] if either buffer was not created by this device
    pub fn albedo_normal_buffer(
        &mut self,
        albedo: Rc<Buffer>,
        normal: Rc<Buffer>,
    ) -> Option<&mut Self> {
        if !self.device.same_device_as(&albedo) || !self.device.same_device_as(&normal) {
            return None;
        }
        self.albedo = Some(albedo);
        self.normal = Some(normal);
        Some(self)
    }

    /// Set whether the color is HDR.
    pub fn hdr(&mut self, hdr: bool) -> &mut Self {
        self.hdr = hdr;
        self
    }

    /// Set whether the auxiliary feature (albedo, normal) images are
    /// noise-free.
    ///
    /// Recommended for highest quality but should not be enabled for noisy
    /// auxiliary images to avoid residual noise.
    pub fn clean_aux(&mut self, clean_aux: bool) -> &mut Self {
        self.clean_aux = clean_aux;
        self
    }

    /// sets the dimensions of the denoising image, if new width * new height
    /// does not equal old width * old height
    pub fn image_dimensions(&mut self, width: usize, height: usize) -> Option<&mut Self> {
        let buffer_dims = self.image_channel_per_pixel * width * height;
        let mut setup_failure = false;
        match &self.albedo {
            None => {}
            Some(buffer) => {
                if buffer.f32_content_size < buffer_dims {
                    self.albedo = None;
                    setup_failure = true;
                }
            }
        }
        match &self.normal {
            None => {}
            Some(buffer) => {
                if buffer.f32_content_size < buffer_dims {
                    self.normal = None;
                    setup_failure = true;
                }
            }
        }
        self.img_dims = (width, height, buffer_dims);
        if setup_failure {
            None
        } else { 
            Some(self) 
        }
    }
    
    pub fn filter_buffer_in_place(&self, color: &Buffer) -> Result<(), Error> {
        self.execute_filter_buffer(None, color)
    }

    fn execute_filter_buffer(&self, color: Option<&Buffer>, output: &Buffer) -> Result<(), Error> {
        let pixel_stride = self.image_channel_per_pixel * size_of::<f32>();
        let row_stride = pixel_stride * self.img_dims.0;
        
        if let Some(alb) = &self.albedo {
            if alb.f32_content_size < self.img_dims.2 {
                return Err(Error::InvalidImageDimensions);
            }
            unsafe {
                oidnSetFilterImage(
                    self.handle,
                    b"albedo\0" as *const _ as _,
                    alb.buffer,
                    OIDNFormat_OIDN_FORMAT_FLOAT3,
                    self.img_dims.0 as _,
                    self.img_dims.1 as _,
                    0,
                    pixel_stride,
                    row_stride,
                );
            }

            // No use supplying normal if albedo was
            // not also given.
            if let Some(norm) = &self.normal {
                if norm.f32_content_size < self.img_dims.2 {
                    return Err(Error::InvalidImageDimensions);
                }
                unsafe {
                    oidnSetFilterImage(
                        self.handle,
                        b"normal\0" as *const _ as _,
                        norm.buffer,
                        OIDNFormat_OIDN_FORMAT_FLOAT3,
                        self.img_dims.0 as _,
                        self.img_dims.1 as _,
                        0,
                        pixel_stride,
                        row_stride,
                    );
                }
            }
        }
        let color_buffer = match color {
            Some(color) => {
                if !self.device.same_device_as(color) {
                    return Err(Error::InvalidArgument);
                }
                if color.f32_content_size < self.img_dims.2 {
                    return Err(Error::InvalidImageDimensions);
                }
                color
            }
            None => {
                if output.f32_content_size < self.img_dims.2 {
                    return Err(Error::InvalidImageDimensions);
                }
                // actually this is a needed borrow, the compiler complains otherwise
                #[allow(clippy::needless_borrow)]
                &output
            }
        };
        unsafe {
            oidnSetFilterImage(
                self.handle,
                b"color\0" as *const _ as _,
                color_buffer.buffer,
                OIDNFormat_OIDN_FORMAT_FLOAT3,
                self.img_dims.0 as _,
                self.img_dims.1 as _,
                0,
                pixel_stride,
                row_stride,
            );
        }
        if !self.device.same_device_as(output) {
            return Err(Error::InvalidArgument);
        }
        if output.f32_content_size < self.img_dims.2 {
            return Err(Error::InvalidImageDimensions);
        }
        unsafe {
            oidnSetFilterImage(
                self.handle,
                b"output\0" as *const _ as _,
                output.buffer,
                OIDNFormat_OIDN_FORMAT_FLOAT3,
                self.img_dims.0 as _,
                self.img_dims.1 as _,
                0,
                pixel_stride,
                row_stride,
            );
            oidnSetFilterBool(self.handle, b"hdr\0" as *const _ as _, self.hdr);
            match self.input_scale {
                None => {}
                Some(input_scale) => {
                    oidnSetFilterFloat(
                        self.handle,
                        b"inputScale\0" as *const _ as _,
                        input_scale,
                    );       
                }
            }
            oidnSetFilterBool(self.handle, b"srgb\0" as *const _ as _, self.srgb);
            oidnSetFilterBool(self.handle, b"clean_aux\0" as *const _ as _, self.clean_aux);

            oidnSetFilterInt(
                self.handle,
                b"quality\0" as *const _ as _,
                self.filter_quality as i32,
            );

            oidnCommitFilter(self.handle);
            oidnExecuteFilter(self.handle);
        }
        Ok(())
    }
}

impl Drop for RayTracing {
    fn drop(&mut self) {
        unsafe {
            oidnReleaseFilter(self.handle);
            oidnReleaseDevice(self.device.0);
        }
    }
}

unsafe impl Send for RayTracing {}
