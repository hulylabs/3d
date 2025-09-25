use crate::geometry::fundamental_constants::COMPONENTS_IN_TEXTURE_COORDINATE;
use crate::material::texture_region::TextureRegion;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
use cgmath::Vector4;

#[repr(i32)]
#[derive(Debug, Copy, Clone)]
pub enum WrapMode {
    Repeat = 0,
    Clamp = 1,
    Discard = 2,
}

#[derive(Debug, Clone)]
pub(crate) struct AtlasRegionMapping {
    area: TextureRegion,
    local_position_to_texture_u: Vector4<f32>,
    local_position_to_texture_v: Vector4<f32>,
    wrap_mode: [WrapMode; COMPONENTS_IN_TEXTURE_COORDINATE],
}

pub struct AtlasRegionMappingBuilder {
    local_position_to_texture_u: Vector4<f32>,
    local_position_to_texture_v: Vector4<f32>,
    wrap_mode: [WrapMode; COMPONENTS_IN_TEXTURE_COORDINATE],
}

impl Default for AtlasRegionMappingBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AtlasRegionMappingBuilder {
    pub fn new() -> Self {
        Self {
            local_position_to_texture_u: Vector4::new(1.0, 0.0, 0.0, 0.0),
            local_position_to_texture_v: Vector4::new(0.0, 1.0, 0.0, 0.0),
            wrap_mode: [WrapMode::Discard; COMPONENTS_IN_TEXTURE_COORDINATE],
        }
    }

    pub fn local_position_to_texture_u(mut self, u_mapping: Vector4<f32>) -> Self {
        self.local_position_to_texture_u = u_mapping;
        self
    }

    pub fn local_position_to_texture_v(mut self, v_mapping: Vector4<f32>) -> Self {
        self.local_position_to_texture_v = v_mapping;
        self
    }

    pub fn wrap_mode(mut self, mode: [WrapMode; COMPONENTS_IN_TEXTURE_COORDINATE]) -> Self {
        self.wrap_mode = mode;
        self
    }

    #[must_use]
    pub(crate) fn build(self, area: TextureRegion) -> AtlasRegionMapping {
        AtlasRegionMapping {
            area,
            local_position_to_texture_u: self.local_position_to_texture_u,
            local_position_to_texture_v: self.local_position_to_texture_v,
            wrap_mode: self.wrap_mode,
        }
    }
}

impl GpuSerializationSize for AtlasRegionMapping {
    const SERIALIZED_QUARTET_COUNT: usize = 4;
}

impl GpuSerializable for AtlasRegionMapping {
    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        container.write_quartet_f32(
            self.area.top_left_corner_uv().x,
            self.area.top_left_corner_uv().y,
            self.area.size().x,
            self.area.size().y,
        );

        container.write_quartet_f32(
            self.local_position_to_texture_u.x,
            self.local_position_to_texture_u.y,
            self.local_position_to_texture_u.z,
            self.local_position_to_texture_u.w,
        );

        container.write_quartet_f32(
            self.local_position_to_texture_v.x,
            self.local_position_to_texture_v.y,
            self.local_position_to_texture_v.z,
            self.local_position_to_texture_v.w,
        );

        container.write_quartet(|writer| {
            writer.write_signed(self.wrap_mode[0] as i32);
            writer.write_signed(self.wrap_mode[1] as i32);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::cast_slice;
    use cgmath::Vector2;
    use rstest::rstest;

    #[must_use]
    fn serialize(system_under_test: AtlasRegionMapping) -> GpuReadySerializationBuffer {
        let mut container = GpuReadySerializationBuffer::new(1, AtlasRegionMapping::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);
        assert!(container.object_fully_written());
        container
    }

    fn assert_region_area(top_left: Vector2<f32>, size: Vector2<f32>, serialized: &[u32]) {
        assert_eq!(f32::from_bits(serialized[0]), top_left.x);
        assert_eq!(f32::from_bits(serialized[1]), top_left.y);
        assert_eq!(f32::from_bits(serialized[2]), size.x);
        assert_eq!(f32::from_bits(serialized[3]), size.y);
    }

    fn assert_edge_mode(serialized: &[u32], u: WrapMode, v: WrapMode,) {
        assert_eq!(i32::from_ne_bytes(serialized[12].to_ne_bytes()), u as i32);
        assert_eq!(i32::from_ne_bytes(serialized[13].to_ne_bytes()), v as i32);
    }

    fn assert_texture_coordinates_mapping(serialized: &[u32], u: Vector4<f32>, v: Vector4<f32>, ) {
        assert_eq!(f32::from_bits(serialized[4]), u.x);
        assert_eq!(f32::from_bits(serialized[5]), u.y);
        assert_eq!(f32::from_bits(serialized[6]), u.z);
        assert_eq!(f32::from_bits(serialized[7]), u.w);

        assert_eq!(f32::from_bits(serialized[ 8]), v.x);
        assert_eq!(f32::from_bits(serialized[ 9]), v.y);
        assert_eq!(f32::from_bits(serialized[10]), v.z);
        assert_eq!(f32::from_bits(serialized[11]), v.w);
    }

    #[test]
    fn test_builder_with_default_values() {
        let expected_top_left = Vector2::new(0.1, 0.2);
        let expected_size = Vector2::new(0.5, 0.6);

        let system_under_test = AtlasRegionMappingBuilder::new().build(TextureRegion::new(expected_top_left, expected_size));

        let container = serialize(system_under_test);
        let serialized: &[u32] = cast_slice(&container.backend());

        assert_region_area(expected_top_left, expected_size, serialized);
        assert_texture_coordinates_mapping(serialized, Vector4::new(1.0, 0.0, 0.0, 0.0), Vector4::new(0.0, 1.0, 0.0, 0.0));
        assert_edge_mode(serialized, WrapMode::Discard, WrapMode::Discard);
    }

    #[test]
    fn test_builder_with_texture_coordinates_mapping() {
        let expected_u_mapping = Vector4::new(1.0, 2.0, 3.0, 4.0);
        let expected_v_mapping = Vector4::new(5.0, 6.0, 7.0, 8.0);

        let system_under_test = AtlasRegionMappingBuilder::new()
            .local_position_to_texture_u(expected_u_mapping)
            .local_position_to_texture_v(expected_v_mapping)
            .build(TextureRegion::new(
                Vector2::new(0.1, 0.2),
                Vector2::new(0.3, 0.4),
            ));

        let container = serialize(system_under_test);
        let serialized: &[u32] = cast_slice(&container.backend());

        assert_texture_coordinates_mapping(serialized, expected_u_mapping, expected_v_mapping);
        assert_edge_mode(serialized, WrapMode::Discard, WrapMode::Discard);
    }

    #[rstest]
    #[case(WrapMode::Repeat, WrapMode::Clamp)]
    #[case(WrapMode::Clamp, WrapMode::Repeat)]
    fn test_builder_with_wrap_modes(#[case] u: WrapMode, #[case] v: WrapMode,) {
        let system_under_test = AtlasRegionMappingBuilder::new()
            .wrap_mode([u, v])
            .build(TextureRegion::new(
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 1.0)
            ));

        let container = serialize(system_under_test);
        let serialized: &[u32] = cast_slice(&container.backend());

        assert_edge_mode(serialized, u, v);
    }

    #[test]
    fn test_builder_with_full_customization() {
        let expected_top_left = Vector2::new(0.1, 0.2);
        let expected_size = Vector2::new(0.3, 0.4);
        let expected_u_mapping = Vector4::new(2.0, 0.0, 0.0, 0.0);
        let expected_v_mapping = Vector4::new(0.0, 3.0, 0.0, 0.0);
        let expected_u_out_of_edge_mode = WrapMode::Repeat;
        let expected_v_out_of_edge_mode = WrapMode::Clamp;

        let system_under_test = AtlasRegionMappingBuilder::new()
            .local_position_to_texture_u(expected_u_mapping)
            .local_position_to_texture_v(expected_v_mapping)
            .wrap_mode([expected_u_out_of_edge_mode, expected_v_out_of_edge_mode])
            .build(TextureRegion::new(
                expected_top_left,
                expected_size
            ));

        let container = serialize(system_under_test);
        let serialized: &[u32] = cast_slice(&container.backend());

        assert_region_area(expected_top_left, expected_size, serialized);
        assert_texture_coordinates_mapping(serialized, expected_u_mapping, expected_v_mapping);
        assert_edge_mode(serialized, expected_u_out_of_edge_mode, expected_v_out_of_edge_mode);
    }
}
