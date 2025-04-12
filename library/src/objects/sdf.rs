use cgmath::num_traits::abs;
use cgmath::SquareMatrix;
use crate::geometry::alias::Vector;
use crate::geometry::transform::Affine;
use crate::objects::material_index::MaterialIndex;
use crate::serialization::filler::{floats_count, GpuFloatBufferFiller};
use crate::serialization::helpers::serialize_matrix;
use crate::serialization::serializable_for_gpu::SerializableForGpu;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SdfBoxIndex(pub(crate) usize);
impl From<usize> for SdfBoxIndex {
    fn from(value: usize) -> Self {
        SdfBoxIndex(value)
    }
}

pub(crate) struct SdfBox {
    location: Affine,
    half_size: Vector,
    corners_radius: f64,
    material_index: MaterialIndex,
}

impl SdfBox {
    #[must_use]
    pub(crate) fn new(location: Affine, half_size: Vector, corners_radius: f64, material_index: MaterialIndex) -> Self {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0);
        assert!(corners_radius >= 0.0);
        assert!(abs(location.determinant()) > 0.0);
        Self { location, half_size, corners_radius, material_index }
    }

    const SERIALIZED_QUARTET_COUNT: usize = 10;
}

impl SerializableForGpu for SdfBox {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(SdfBox::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= SdfBox::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;
        serialize_matrix(container, &self.location, &mut index);
        serialize_matrix(container, &self.location.invert().unwrap(), &mut index);

        container.write_and_move_next(self.half_size.x, &mut index);
        container.write_and_move_next(self.half_size.y, &mut index);
        container.write_and_move_next(self.half_size.z, &mut index);
        container.write_and_move_next(self.corners_radius, &mut index);

        container.write_and_move_next(self.material_index.as_f64(), &mut index);
        container.pad_to_align(&mut index);

        debug_assert_eq!(index, SdfBox::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::helpers::MATRIX_FLOATS_COUNT;

    #[test]
    fn test_sdf_box_serialize_into() {
        let expected_location = Affine::from_nonuniform_scale(0.5, 0.6, 0.7);
        let expected_half_size = Vector { x: 1.0, y: 2.0, z: 3.0 };
        let expected_corners_radius = 0.9;
        let expected_material_index = MaterialIndex(4);

        let system_under_test = SdfBox::new(expected_location, expected_half_size, expected_corners_radius, expected_material_index);

        let mut container = vec![0.0; SdfBox::SERIALIZED_SIZE_FLOATS];
        system_under_test.serialize_into(&mut container);

        let location_serialized = {
            let mut location_serialized = vec![0.0_f32; MATRIX_FLOATS_COUNT * 2];
            let mut counter = 0;
            serialize_matrix(&mut location_serialized, &expected_location, &mut counter);
            serialize_matrix(&mut location_serialized, &expected_location.invert().unwrap(), &mut counter);
            location_serialized
        };

        let mut values_checked = 0;
        assert_eq!(&container[values_checked..values_checked + MATRIX_FLOATS_COUNT], &location_serialized);
        values_checked += MATRIX_FLOATS_COUNT;

        assert_eq!(container[values_checked], expected_half_size.x as f32);
        values_checked += 1;
        assert_eq!(container[values_checked], expected_half_size.y as f32);
        values_checked += 1;
        assert_eq!(container[values_checked], expected_half_size.z as f32);
        values_checked += 1;
        assert_eq!(container[values_checked], expected_corners_radius as f32);
        values_checked += 1;

        assert_eq!(container[values_checked], expected_material_index.as_f64() as f32);
        values_checked += 1;
        assert_eq!(container[values_checked], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
        values_checked += 1;
        assert_eq!(container[values_checked], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
        values_checked += 1;
        assert_eq!(container[values_checked], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
        values_checked += 1;

        assert_eq!(values_checked, SdfBox::SERIALIZED_SIZE_FLOATS);
    }
}