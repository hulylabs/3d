//! Rust bindings to Intel's
//! [Open Image Denoise](https://github.com/OpenImageDenoise/oidn).
//!
//! Open Image Denoise documentation can be found
//! [here](https://openimagedenoise.github.io/documentation.html).

use crate::denoiser::filter::Quality;
use crate::denoiser::buffer::Buffer;
use crate::denoiser::device::Device;
use crate::denoiser::filter::RayTracing;
use log::error;
use std::rc::Rc;


const CHANNELS_PER_PIXEL: usize = 4;

#[must_use]
fn image_f32_size(width: usize, height: usize) -> usize {
    width * height * CHANNELS_PER_PIXEL
}

struct Storage { // TODO: is it faster to use single huge buffer with offsets?
    beauty_io_image: Rc<Buffer>,
    aux_input_albedo: Rc<Buffer>,
    aux_input_normals: Rc<Buffer>,
}

impl Storage {
    #[must_use]
    fn new(device: Rc<Device>, width: usize, height: usize) -> Option<Self> {
        assert!(width > 0);
        assert!(height > 0);
        
        let f32_count = image_f32_size(width, height);
        
        let beauty_io_image = device.create_buffer(f32_count)?;
        let aux_input_albedo = device.create_buffer(f32_count)?;
        let aux_input_normals = device.create_buffer(f32_count)?;
        
        Some(Self {
            beauty_io_image: Rc::new(beauty_io_image), 
            aux_input_albedo: Rc::new(aux_input_albedo), 
            aux_input_normals: Rc::new(aux_input_normals), 
        })
    }
    
    #[must_use]
    fn pixel_count(&self) -> usize {
        self.beauty_io_image.f32_content_size / CHANNELS_PER_PIXEL
    }
}

pub(crate) struct DenoiserExecutor<'a> {
    device: Rc<Device>,
    filter: &'a mut RayTracing,
    
    storage: Rc<Storage>,
    image_width: usize,
    image_height: usize,
    
    albedo_write_issued: bool,
    normal_write_issued: bool,
    noisy_beauty_write_issued: bool,
}

impl DenoiserExecutor<'_> {
    pub(crate) fn issue_albedo_write(&mut self, albedo: &[f32]) {
        assert!(!self.albedo_write_issued);
        self.albedo_write_issued = true;
        self.issue_write(self.storage.aux_input_albedo.clone(), albedo, "albedo");
    }

    pub(crate) fn issue_normal_write(&mut self, normal: &[f32]) {
        assert!(!self.normal_write_issued);
        self.normal_write_issued = true;
        self.issue_write(self.storage.aux_input_normals.clone(), normal, "normal");
    }

    pub(crate) fn issue_noisy_beauty_write(&mut self, noisy_pixels: &[f32]) {
        assert!(!self.noisy_beauty_write_issued);
        self.noisy_beauty_write_issued = true;
        self.issue_write(self.storage.beauty_io_image.clone(), noisy_pixels, "noisy beauty");
    }

    fn issue_write(&self, buffer: Rc<Buffer>, data: &[f32], what: &str) {
        let f32_image_size = image_f32_size(self.image_width, self.image_height);
        assert!(data.len() >= f32_image_size);
        buffer.write_async(&data[..f32_image_size]).unwrap_or_else(|| panic!("failed to issue {} write", what))
    }

    pub(crate) fn filter(&mut self, denoised_pixels: &mut [f32]) {
        let image_f32_size = image_f32_size(self.image_width, self.image_height);
        assert!(denoised_pixels.len() >= image_f32_size);
        assert!(self.noisy_beauty_write_issued);
        
        self.filter
            .filter_buffer_in_place(self.storage.beauty_io_image.as_ref())
            .expect("denoise execution failure");

        if let Err(e) = self.device.get_error() {
            error!("error denoising image: {:?}, {}", e.0, e.1);
        }
        
        self.storage.beauty_io_image.read_slice_into(image_f32_size, denoised_pixels).expect("failed to read denoised data back");
    }
}

pub(crate) struct Denoiser {
    device: Rc<Device>,
    filter: RayTracing,
    
    storage: Option<Rc<Storage>>,
}

impl Denoiser {
    
    #[must_use]
    pub(crate) fn new() -> Self {
        let device = Rc::new(Device::new());
        let filter = RayTracing::new(device.clone(), CHANNELS_PER_PIXEL);
                
        let mut result = Self { 
            device: device.clone(), 
            filter,
            storage: None,
        };
        
        result.filter
            .clean_aux(true)
            .hdr(true)
            .filter_quality(Quality::High);

        result
    }
    
    #[must_use]
    pub(crate) fn begin_denoise(&mut self, width: usize, height: usize) -> DenoiserExecutor {
        assert!(width > 0);
        assert!(height > 0);

        let storage: Rc<Storage> = self.get_storage(width, height);

        self.filter
            .image_dimensions(width, height)
            .expect("denoise filter dimensions setup error")
        ;
        
        DenoiserExecutor {
            device: self.device.clone(),
            filter: &mut self.filter,
            storage: storage.clone(),
            image_width: width,
            image_height: height,
            
            albedo_write_issued: false,
            normal_write_issued: false,
            noisy_beauty_write_issued: false,
        }
    }

    #[must_use]
    fn get_storage(&mut self, width: usize, height: usize) -> Rc<Storage> {
        assert!(width > 0);
        assert!(height > 0);
        let desired_pixel_count = width * height;
        match self.storage.as_ref() {
            Some(storage) => {
                if storage.pixel_count() < desired_pixel_count {
                    self.realloc_storage(width, height)
                } else {
                    storage.clone()
                }
            },
            None => {
                self.realloc_storage(width, height)
            }
        }
    }

    #[must_use]
    fn realloc_storage(&mut self, width: usize, height: usize) -> Rc<Storage> {
        assert!(width > 0);
        assert!(height > 0);
        
        let storage 
            = Storage::new(self.device.clone(), width, height)
                .expect("failed to allocate denoiser storage");
        let result = Rc::new(storage);
        
        self.storage = Some(result.clone());

        self.filter
            .albedo_normal_buffer(result.aux_input_albedo.clone(), result.aux_input_normals.clone())
            .expect("denoise aux buffers configuration error")
        ;
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_f32_size() {
        assert_eq!(image_f32_size(1, 1), 4);
        assert_eq!(image_f32_size(2, 2), 16);
        assert_eq!(image_f32_size(0, 10), 0);
    }

    #[test]
    fn test_storage_new_and_pixel_count() {
        let device = Rc::new(Device::new());
        let width = 17;
        let height = 5;

        let system_under_test = Storage::new(device, width, height).expect("storage should be created");
        let expected_pixel_count = width * height;

        assert_eq!(system_under_test.pixel_count(), expected_pixel_count);
    }

    #[test]
    fn test_storage_new_and_buffer_sizes() {
        let device = Rc::new(Device::new());
        let width = 13;
        let height = 5;

        let system_under_test = Storage::new(device, width, height).expect("storage should be created");
        
        assert_eq!(system_under_test.beauty_io_image.f32_content_size, system_under_test.aux_input_albedo.f32_content_size);
        assert_eq!(system_under_test.beauty_io_image.f32_content_size, system_under_test.aux_input_normals.f32_content_size);
        assert_eq!(system_under_test.beauty_io_image.f32_content_size, image_f32_size(width, height));
    }

    #[test]
    #[should_panic]
    fn test_storage_new_zero_width_panics() {
        let _ = Storage::new(Rc::new(Device::new()), 0, 10);
    }

    #[test]
    #[should_panic]
    fn test_storage_new_zero_height_panics() {
        let _ = Storage::new(Rc::new(Device::new()), 10, 0);
    }

    #[test]
    fn test_denoiser_construction() {
        let _ = Denoiser::new();
    }
}