use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
use palette::Srgb;
use strum_macros::{EnumCount, EnumIter};
use crate::material::texture_reference::TextureReference;

#[derive(Copy, Clone, Debug, PartialEq, EnumCount, EnumIter)]
#[repr(i32)]
pub enum MaterialClass {
    Lambert,
    Mirror,
    Glass,
}

impl Default for MaterialClass {
    fn default() -> Self {
        Self::Lambert
    }
}

impl MaterialClass {
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct MaterialProperties {
    albedo: Srgb,
    specular: Srgb,
    emission: Srgb,
    specular_strength: f64,
    roughness: f64,
    refractive_index_eta: f64,
    albedo_texture: TextureReference,
    class: MaterialClass,
}

impl MaterialProperties {
    const ZERO_COLOR: Srgb = Srgb::new(0.0, 0.0, 0.0);

    #[must_use]
    pub fn new() -> Self {
        Self { ..Self::default() }
    }

    #[must_use]
    pub fn albedo_texture(&self) -> TextureReference {
        self.albedo_texture
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

    pub fn with_albedo_texture(mut self, reference: TextureReference) -> Self {
        self.albedo_texture = reference;
        self
    }
}

impl GpuSerializationSize for MaterialProperties {
    const SERIALIZED_QUARTET_COUNT: usize = 4;
}

impl GpuSerializable for MaterialProperties {
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
        container.write_quartet(|writer| {
            writer.write_float_64(self.roughness);
            writer.write_float_64(self.refractive_index_eta);
            writer.write_signed(self.albedo_texture.as_gpu_readable_index());
            writer.write_signed(self.class.as_i32());
        });

        debug_assert!(container.object_fully_written());
    }
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            albedo: Self::ZERO_COLOR,
            specular: Self::ZERO_COLOR,
            emission: Self::ZERO_COLOR,
            specular_strength: 0.0,
            roughness: 0.0,
            refractive_index_eta: 0.0,
            albedo_texture: TextureReference::None,
            class: MaterialClass::Lambert,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;
    use bytemuck::cast_slice;
    use strum::IntoEnumIterator;
    use crate::material::procedural_texture_index::ProceduralTextureUid;

    #[test]
    fn test_serialize_into() {
        let expected_albedo = Srgb::new(0.5, 0.6, 0.7);
        let expected_specular = Srgb::new(0.8, 0.9, 1.0);
        let expected_emission = Srgb::new(1.1, 2.2, 3.3);
        let expected_specular_strength = 0.5;
        let expected_roughness = 0.7;
        let expected_refractive_index = 1.5;
        let expected_class = MaterialClass::Glass;
        let expected_texture_reference = TextureReference::Procedural(ProceduralTextureUid(13));
        
        let system_under_test = MaterialProperties::new()
            .with_albedo(expected_albedo.red, expected_albedo.green, expected_albedo.blue)
            .with_specular(expected_specular.red, expected_specular.green, expected_specular.blue)
            .with_emission(expected_emission.red, expected_emission.green, expected_emission.blue)
            .with_specular_strength(expected_specular_strength)
            .with_roughness(expected_roughness)
            .with_refractive_index_eta(expected_refractive_index)
            .with_albedo_texture(expected_texture_reference)
            .with_class(expected_class);

        let mut container = GpuReadySerializationBuffer::new(1, MaterialProperties::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);

        let serialized: &[u32] = cast_slice(&container.backend());

        assert_eq!(f32::from_bits(serialized[ 0]), expected_albedo.red);
        assert_eq!(f32::from_bits(serialized[ 1]), expected_albedo.green);
        assert_eq!(f32::from_bits(serialized[ 2]), expected_albedo.blue);
        assert_eq!(f32::from_bits(serialized[ 3]), DEFAULT_PAD_VALUE);

        assert_eq!(f32::from_bits(serialized[ 4]), expected_specular.red);
        assert_eq!(f32::from_bits(serialized[ 5]), expected_specular.green);
        assert_eq!(f32::from_bits(serialized[ 6]), expected_specular.blue);
        assert_eq!(f32::from_bits(serialized[ 7]), DEFAULT_PAD_VALUE);

        assert_eq!(f32::from_bits(serialized[ 8]),  expected_emission.red);
        assert_eq!(f32::from_bits(serialized[ 9]),  expected_emission.green);
        assert_eq!(f32::from_bits(serialized[10]), expected_emission.blue);
        assert_eq!(f32::from_bits(serialized[11]), expected_specular_strength as f32);

        assert_eq!(f32::from_bits(serialized[12]), expected_roughness as f32);
        assert_eq!(f32::from_bits(serialized[13]), expected_refractive_index as f32);
        assert_eq!(i32::from_ne_bytes(serialized[14].to_ne_bytes()), expected_texture_reference.as_gpu_readable_index());
        assert_eq!(i32::from_ne_bytes(serialized[15].to_ne_bytes()), expected_class.as_i32());
    }

    #[test]
    fn test_material_class_as_i32() {
        for system_under_test in MaterialClass::iter()  {
            let value = system_under_test.as_i32();
            assert_eq!(value, system_under_test as i32);
        }
    }

    #[test]
    fn test_material_default() {
        let system_under_test = MaterialProperties::default();
        assert_eq!(system_under_test.albedo, MaterialProperties::ZERO_COLOR);
        assert_eq!(system_under_test.specular, MaterialProperties::ZERO_COLOR);
        assert_eq!(system_under_test.emission, MaterialProperties::ZERO_COLOR);
        assert_eq!(system_under_test.specular_strength, 0.0);
        assert_eq!(system_under_test.roughness, 0.0);
        assert_eq!(system_under_test.refractive_index_eta, 0.0);
        assert_eq!(system_under_test.class, MaterialClass::Lambert);
    }

    #[test]
    fn test_material_with_albedo() {
        let expected_albedo = Srgb::new(0.5, 0.6, 0.7);
        let system_under_test = MaterialProperties::default().with_albedo(expected_albedo.red, expected_albedo.green, expected_albedo.blue);
        assert_eq!(system_under_test, MaterialProperties { albedo: expected_albedo, ..Default::default() });
    }

    #[test]
    fn test_material_with_specular() {
        let expected_specular = Srgb::new(0.8, 0.9, 1.0);
        let system_under_test = MaterialProperties::default().with_specular(expected_specular.red, expected_specular.green, expected_specular.blue);
        assert_eq!(system_under_test, MaterialProperties { specular: expected_specular, ..Default::default() });
    }

    #[test]
    fn test_material_with_emission() {
        let expected_emission = Srgb::new(1.1, 2.2, 3.3);
        let system_under_test = MaterialProperties::default().with_emission(expected_emission.red, expected_emission.green, expected_emission.blue);
        assert_eq!(system_under_test, MaterialProperties { emission: expected_emission, ..Default::default() });
    }

    #[test]
    fn test_material_with_specular_strength() {
        let expected_specular_strength = 0.5;
        let system_under_test = MaterialProperties::default().with_specular_strength(expected_specular_strength);
        assert_eq!(system_under_test, MaterialProperties { specular_strength: expected_specular_strength, ..Default::default() });
    }

    #[test]
    fn test_material_with_roughness() {
        let expected_roughness = 0.7;
        let system_under_test = MaterialProperties::default().with_roughness(expected_roughness);
        assert_eq!(system_under_test, MaterialProperties { roughness: expected_roughness, ..Default::default() });
    }

    #[test]
    fn test_material_with_refractive_index_eta() {
        let expected_refractive_index = 1.5;
        let system_under_test = MaterialProperties::default().with_refractive_index_eta(expected_refractive_index);
        assert_eq!(system_under_test, MaterialProperties { refractive_index_eta: expected_refractive_index, ..Default::default() });
    }

    #[test]
    fn test_material_with_class() {
        let expected_class = MaterialClass::Glass;
        let system_under_test = MaterialProperties::default().with_class(expected_class);
        assert_eq!(system_under_test, MaterialProperties { class: expected_class, ..Default::default() });
    }
}