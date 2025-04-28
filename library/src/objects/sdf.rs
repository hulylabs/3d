use crate::geometry::alias::Vector;
use crate::geometry::transform::Affine;
use crate::objects::common_properties::Linkage;
use crate::objects::material_index::MaterialIndex;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::helpers::serialize_matrix;
use crate::objects::ray_traceable::RayTraceable;
use cgmath::num_traits::abs;
use cgmath::SquareMatrix;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};

pub(crate) struct SdfBox {
    location: Affine,
    half_size: Vector,
    corners_radius: f64,
    links: Linkage,
}

impl SdfBox {
    #[must_use]
    pub(crate) fn new(location: Affine, half_size: Vector, corners_radius: f64, links: Linkage) -> Self {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0);
        assert!(corners_radius >= 0.0);
        assert!(abs(location.determinant()) > 0.0);
        Self { location, half_size, corners_radius, links }
    }
}

impl GpuSerializationSize for SdfBox {
    const SERIALIZED_QUARTET_COUNT: usize = 10;
}

impl GpuSerializable for SdfBox {
    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        serialize_matrix(container, &self.location);
        serialize_matrix(container, &self.location.invert().unwrap());

        container.write_quartet_f64(
            self.half_size.x,
            self.half_size.y,
            self.half_size.z,
            self.corners_radius,
        );

        container.write_quartet(|writer| {
            writer.write_float(self.links.material_index().0 as f32);
            writer.write_integer(self.links.uid().0);
        });

        debug_assert!(container.object_fully_written());
    }
}

impl RayTraceable for SdfBox {
    #[must_use]
    fn material(&self) -> MaterialIndex {
        self.links.material_index()
    }

    fn set_material(&mut self, new_material_index: MaterialIndex) {
        self.links.set_material_index(new_material_index)
    }

    #[must_use]
    fn serialized_quartet_count(&self) -> usize {
        SdfBox::SERIALIZED_QUARTET_COUNT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::transform::constants::MATRIX_FLOATS_COUNT;
    use crate::objects::material_index::MaterialIndex;
    use crate::serialization::gpu_ready_serialization_buffer::{DEFAULT_PAD_VALUE, ELEMENTS_IN_QUARTET};
    use crate::utils::object_uid::ObjectUid;
    use bytemuck::cast_slice;

    #[must_use]
    fn matrix_at(index: usize, matrix: &Affine) -> f32 {
        let matrix_side_size = 4;
        let column = index / matrix_side_size;
        let row = index % matrix_side_size;
        matrix[column][row] as f32
    }

    #[test]
    fn test_sdf_box_serialize_into() {
        let expected_location = Affine::from_nonuniform_scale(0.5, 0.6, 0.7);
        let expected_half_size = Vector { x: 1.0, y: 2.0, z: 3.0 };
        let expected_corners_radius = 0.9;
        let expected_material_index = MaterialIndex(4);
        let expected_object_uid = ObjectUid(7);
        
        let system_under_test = SdfBox::new(expected_location, expected_half_size, expected_corners_radius, Linkage::new(expected_object_uid, expected_material_index));

        let mut container = GpuReadySerializationBuffer::new(1, SdfBox::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);

        let location_serialized = {
            let mut location_serialized = vec![0.0_f32; MATRIX_FLOATS_COUNT * 2];
            let mut counter = 0;
            for i in 0..MATRIX_FLOATS_COUNT {
                location_serialized[counter] = matrix_at(i, &expected_location);
                counter += 1;
            }
            let inverted_location = expected_location.invert().unwrap();
            for i in 0..MATRIX_FLOATS_COUNT {
                location_serialized[counter] = matrix_at(i, &inverted_location);
                counter += 1;
            }

            location_serialized
        };

        let serialized: &[f32] = cast_slice(&container.backend());

        let mut values_checked = 0;
        assert_eq!(&serialized[values_checked..values_checked + MATRIX_FLOATS_COUNT], &location_serialized[0..MATRIX_FLOATS_COUNT]);
        values_checked += MATRIX_FLOATS_COUNT;
        assert_eq!(&serialized[values_checked..values_checked + MATRIX_FLOATS_COUNT], &location_serialized[MATRIX_FLOATS_COUNT..MATRIX_FLOATS_COUNT * 2]);
        values_checked += MATRIX_FLOATS_COUNT;

        assert_eq!(serialized[values_checked], expected_half_size.x as f32);
        values_checked += 1;
        assert_eq!(serialized[values_checked], expected_half_size.y as f32);
        values_checked += 1;
        assert_eq!(serialized[values_checked], expected_half_size.z as f32);
        values_checked += 1;
        assert_eq!(serialized[values_checked], expected_corners_radius as f32);
        values_checked += 1;

        assert_eq!(serialized[values_checked], expected_material_index.as_f64() as f32);
        values_checked += 1;
        assert_eq!(serialized[values_checked].to_bits(), expected_object_uid.0);
        values_checked += 1;
        assert_eq!(serialized[values_checked], DEFAULT_PAD_VALUE);
        values_checked += 1;
        assert_eq!(serialized[values_checked], DEFAULT_PAD_VALUE);
        values_checked += 1;

        assert_eq!(values_checked, SdfBox::SERIALIZED_QUARTET_COUNT * ELEMENTS_IN_QUARTET);
    }
}