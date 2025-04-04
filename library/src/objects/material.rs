use crate::serialization::helpers::{GpuFloatBufferFiller, floats_count};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use palette::Srgb;

#[derive(Copy, Clone)]
pub enum MaterialClass {
    Lambert,
    Mirror,
    Glass,
    Isotropic,
}

impl Default for MaterialClass {
    #[must_use]
    fn default() -> Self {
        MaterialClass::Lambert
    }
}

impl MaterialClass {
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        (self as u32) as f64
    }
}

#[derive(Copy, Clone)]
pub struct Material {
    albedo: Srgb,
    specular: Srgb,
    emission: Srgb,
    specular_strength: f64,
    roughness: f64,
    refractive_index_eta: f64,
    class: MaterialClass,
}

impl Material {
    const SERIALIZED_QUARTET_COUNT: usize = 4;

    #[must_use]
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn with_albedo(mut self, r: f32, g: f32, b: f32) -> Self {
        assert!(r >= 0.0 && r <= 1.0);
        assert!(g >= 0.0 && g <= 1.0);
        assert!(b >= 0.0 && b <= 1.0);
        self.albedo = Srgb::new(r, g, b);
        self
    }

    pub fn with_specular(mut self, r: f32, g: f32, b: f32) -> Self {
        assert!(r >= 0.0 && r <= 1.0);
        assert!(g >= 0.0 && g <= 1.0);
        assert!(b >= 0.0 && b <= 1.0);
        self.specular = Srgb::new(r, g, b);
        self
    }

    pub fn with_emission(mut self, r: f32, g: f32, b: f32) -> Self {
        assert!(r >= 0.0);
        assert!(g >= 0.0);
        assert!(b >= 0.0);
        self.emission = Srgb::new(r, g, b);
        self
    }

    pub fn with_specular_strength(mut self, specular_strength: f64) -> Self {
        self.specular_strength = specular_strength;
        self
    }

    pub fn with_roughness(mut self, roughness: f64) -> Self {
        self.roughness = roughness;
        self
    }

    pub fn with_refractive_index_eta(mut self, refractive_index_eta: f64) -> Self {
        self.refractive_index_eta = refractive_index_eta;
        self
    }

    pub fn with_class(mut self, class: MaterialClass) -> Self {
        self.class = class;
        self
    }
}

impl Default for Material {
    #[must_use]
    fn default() -> Self {
        Material {
            albedo: Srgb::new(0.0, 0.0, 0.0),
            specular: Srgb::new(0.0, 0.0, 0.0),
            emission: Srgb::new(0.0, 0.0, 0.0),
            specular_strength: 0.0,
            roughness: 0.0,
            refractive_index_eta: 0.0,
            class: MaterialClass::Lambert,
        }
    }
}

impl SerializableForGpu for Material {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Material::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Material::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;
        container.write_and_move_next(self.albedo.red as f64, &mut index);
        container.write_and_move_next(self.albedo.green as f64, &mut index);
        container.write_and_move_next(self.albedo.blue as f64, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.specular.red as f64, &mut index);
        container.write_and_move_next(self.specular.green as f64, &mut index);
        container.write_and_move_next(self.specular.blue as f64, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.emission.red as f64, &mut index);
        container.write_and_move_next(self.emission.green as f64, &mut index);
        container.write_and_move_next(self.emission.blue as f64, &mut index);
        container.write_and_move_next(self.specular_strength, &mut index);

        container.write_and_move_next(self.roughness, &mut index);
        container.write_and_move_next(self.refractive_index_eta, &mut index);
        container.write_and_move_next(self.class.as_f64(), &mut index);
        container.pad_to_align(&mut index);

        assert_eq!(index, Material::SERIALIZED_SIZE_FLOATS);
    }
}

// TODO: unit tests