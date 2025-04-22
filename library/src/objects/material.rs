use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use palette::Srgb;
use strum_macros::{EnumCount, EnumIter};
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};

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
    const ZERO_COLOR: Srgb = Srgb::new(0.0, 0.0, 0.0);

    #[must_use]
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    pub fn with_albedo(mut self, r: f32, g: f32, b: f32) -> Self {
        assert!(r >= 0.0);
        assert!(g >= 0.0);
        assert!(b >= 0.0);
        self.albedo = Srgb::new(r, g, b);
        self
    }

    pub fn with_specular(mut self, r: f32, g: f32, b: f32) -> Self {
        assert!(r >= 0.0);
        assert!(g >= 0.0);
        assert!(b >= 0.0);
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

impl GpuSerializationSize for Material {
    const SERIALIZED_QUARTET_COUNT: usize = 4;
}

impl GpuSerializable for Material {
    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        container.write_padded_quartet_f32(
            self.albedo.red,
            self.albedo.green,
            self.albedo.blue,
        );
        container.write_padded_quartet_f32(
            self.specular.red,
            self.specular.green,
            self.specular.blue,
        );
        container.write_quartet_f32(
            self.emission.red,
            self.emission.green,
            self.emission.blue,
            self.specular_strength as f32,
        );
        container.write_padded_quartet_f64(
            self.roughness,
            self.refractive_index_eta,
            self.class.as_f64(),
        );

        debug_assert!(container.object_fully_written());
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

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::cast_slice;
    use strum::IntoEnumIterator;
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;

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

        let mut container = GpuReadySerializationBuffer::new(1, Material::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);

        let serialized: &[f32] = cast_slice(&container.backend());

        assert_eq!(serialized[ 0], expected_albedo.red);
        assert_eq!(serialized[ 1], expected_albedo.green);
        assert_eq!(serialized[ 2], expected_albedo.blue);
        assert_eq!(serialized[ 3], DEFAULT_PAD_VALUE);

        assert_eq!(serialized[ 4], expected_specular.red);
        assert_eq!(serialized[ 5], expected_specular.green);
        assert_eq!(serialized[ 6], expected_specular.blue);
        assert_eq!(serialized[ 7], DEFAULT_PAD_VALUE);

        assert_eq!(serialized[ 8],  expected_emission.red);
        assert_eq!(serialized[ 9],  expected_emission.green);
        assert_eq!(serialized[10], expected_emission.blue);
        assert_eq!(serialized[11], expected_specular_strength as f32);

        assert_eq!(serialized[12], expected_roughness as f32);
        assert_eq!(serialized[13], expected_refractive_index as f32);
        assert_eq!(serialized[14], expected_class.as_f64() as f32);
        assert_eq!(serialized[15], DEFAULT_PAD_VALUE);
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