use crate::serialization::filler::{floats_count, GpuFloatBufferFiller};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use palette::Srgb;
use strum_macros::{EnumCount, EnumIter};

#[derive(Copy, Clone, Debug, PartialEq, EnumCount, EnumIter)]
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

#[derive(Copy, Clone, PartialEq, Debug)]
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

    const ZERO_COLOR: Srgb = Srgb::new(0.0, 0.0, 0.0);

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
            albedo: Self::ZERO_COLOR,
            specular: Self::ZERO_COLOR,
            emission: Self::ZERO_COLOR,
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

        debug_assert_eq!(index, Material::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_serialize_into() {
        let expected_albedo = Srgb::new(0.5, 0.6, 0.7);
        let expected_specular = Srgb::new(0.8, 0.9, 1.0);
        let expected_emission = Srgb::new(1.1, 2.2, 3.3);
        let expected_specular_strength = 0.5;
        let expected_roughness = 0.7;
        let expected_refractive_index = 1.5;
        let expected_class = MaterialClass::Glass;

        let system_under_test = Material::new()
            .with_albedo(expected_albedo.red, expected_albedo.green, expected_albedo.blue)
            .with_specular(expected_specular.red, expected_specular.green, expected_specular.blue)
            .with_emission(expected_emission.red, expected_emission.green, expected_emission.blue)
            .with_specular_strength(expected_specular_strength)
            .with_roughness(expected_roughness)
            .with_refractive_index_eta(expected_refractive_index)
            .with_class(expected_class);

        let buffer_initial_filler = -1.0;

        let mut container = vec![buffer_initial_filler; Material::SERIALIZED_SIZE_FLOATS + 1];
        system_under_test.serialize_into(&mut container);

        assert_eq!(container[0], expected_albedo.red);
        assert_eq!(container[1], expected_albedo.green);
        assert_eq!(container[2], expected_albedo.blue);
        assert_eq!(container[3], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);

        assert_eq!(container[4], expected_specular.red);
        assert_eq!(container[5], expected_specular.green);
        assert_eq!(container[6], expected_specular.blue);
        assert_eq!(container[7], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);

        assert_eq!(container[8],  expected_emission.red);
        assert_eq!(container[9],  expected_emission.green);
        assert_eq!(container[10], expected_emission.blue);
        assert_eq!(container[11], expected_specular_strength as f32);

        assert_eq!(container[12], expected_roughness as f32);
        assert_eq!(container[13], expected_refractive_index as f32);
        assert_eq!(container[14], expected_class.as_f64() as f32);
        assert_eq!(container[15], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
    }

    #[test]
    fn test_material_class_as_f64() {
        for system_under_test in MaterialClass::iter()  {
            let value = system_under_test.as_f64();
            assert_eq!(value, value as usize as f64);
        }
    }

    #[test]
    fn test_material_default() {
        let system_under_test = Material::default();
        assert_eq!(system_under_test.albedo, Material::ZERO_COLOR);
        assert_eq!(system_under_test.specular, Material::ZERO_COLOR);
        assert_eq!(system_under_test.emission, Material::ZERO_COLOR);
        assert_eq!(system_under_test.specular_strength, 0.0);
        assert_eq!(system_under_test.roughness, 0.0);
        assert_eq!(system_under_test.refractive_index_eta, 0.0);
        assert_eq!(system_under_test.class, MaterialClass::Lambert);
    }

    #[test]
    fn test_material_with_albedo() {
        let expected_albedo = Srgb::new(0.5, 0.6, 0.7);
        let system_under_test = Material::default().with_albedo(expected_albedo.red, expected_albedo.green, expected_albedo.blue);
        assert_eq!(system_under_test, Material { albedo: expected_albedo, ..Default::default() });
    }

    #[test]
    fn test_material_with_specular() {
        let expected_specular = Srgb::new(0.8, 0.9, 1.0);
        let system_under_test = Material::default().with_specular(expected_specular.red, expected_specular.green, expected_specular.blue);
        assert_eq!(system_under_test, Material { specular: expected_specular, ..Default::default() });
    }

    #[test]
    fn test_material_with_emission() {
        let expected_emission = Srgb::new(1.1, 2.2, 3.3);
        let system_under_test = Material::default().with_emission(expected_emission.red, expected_emission.green, expected_emission.blue);
        assert_eq!(system_under_test, Material { emission: expected_emission, ..Default::default() });
    }

    #[test]
    fn test_material_with_specular_strength() {
        let expected_specular_strength = 0.5;
        let system_under_test = Material::default().with_specular_strength(expected_specular_strength);
        assert_eq!(system_under_test, Material { specular_strength: expected_specular_strength, ..Default::default() });
    }

    #[test]
    fn test_material_with_roughness() {
        let expected_roughness = 0.7;
        let system_under_test = Material::default().with_roughness(expected_roughness);
        assert_eq!(system_under_test, Material { roughness: expected_roughness, ..Default::default() });
    }

    #[test]
    fn test_material_with_refractive_index_eta() {
        let expected_refractive_index = 1.5;
        let system_under_test = Material::default().with_refractive_index_eta(expected_refractive_index);
        assert_eq!(system_under_test, Material { refractive_index_eta: expected_refractive_index, ..Default::default() });
    }

    #[test]
    fn test_material_with_class() {
        let expected_class = MaterialClass::Glass;
        let system_under_test = Material::default().with_class(expected_class);
        assert_eq!(system_under_test, Material { class: expected_class, ..Default::default() });
    }
}