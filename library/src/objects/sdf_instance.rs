use crate::geometry::transform::Affine;
use crate::geometry::utils::is_affine;
use crate::objects::common_properties::Linkage;
use crate::objects::material_index::MaterialIndex;
use crate::objects::ray_traceable::RayTraceable;
use crate::objects::sdf_class_index::SdfClassIndex;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
use crate::serialization::serialize_matrix::serialize_matrix_3x4;
use cgmath::num_traits::abs;
use cgmath::SquareMatrix;
use more_asserts::assert_gt;

pub(crate) struct SdfInstance {
    location: Affine,
    ray_marching_step_scale: f64,
    class: SdfClassIndex,
    links: Linkage,
}

impl SdfInstance {
    #[must_use]
    pub(crate) fn new(location: Affine, ray_marching_step_scale: f64, class: SdfClassIndex, links: Linkage) -> Self {
        assert_gt!(abs(location.determinant()), 0.0, "location should not change basis orientation, or ray marching will break");
        assert_gt!(ray_marching_step_scale, 0.0);
        assert!(is_affine(&location), "projection matrices are not supported");
        Self { location, ray_marching_step_scale, class, links }
    }
}

impl GpuSerializationSize for SdfInstance {
    const SERIALIZED_QUARTET_COUNT: usize = 7;
}

impl GpuSerializable for SdfInstance {
    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        serialize_matrix_3x4(container, &self.location);
        serialize_matrix_3x4(container, &self.location.invert().unwrap());

        container.write_quartet(|writer| {
            writer.write_float_64(self.ray_marching_step_scale);
            writer.write_float_64(self.class.as_f64());
            writer.write_unsigned(self.links.material_index().0 as u32);
            writer.write_unsigned(self.links.uid().0);
        });

        debug_assert!(container.object_fully_written());
    }
}

impl RayTraceable for SdfInstance {
    #[must_use]
    fn material(&self) -> MaterialIndex {
        self.links.material_index()
    }

    fn set_material(&mut self, new_material_index: MaterialIndex) {
        self.links.set_material_index(new_material_index)
    }

    #[must_use]
    fn serialized_quartet_count(&self) -> usize {
        SdfInstance::SERIALIZED_QUARTET_COUNT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::transform::constants::MATRIX_FLOATS_COUNT;
    use crate::objects::material_index::MaterialIndex;
    use crate::serialization::gpu_ready_serialization_buffer::ELEMENTS_IN_QUARTET;
    use crate::utils::object_uid::ObjectUid;
    use bytemuck::cast_slice;

    #[test]
    fn test_sdf_box_serialize_into() {
        let expected_location = Affine::from_nonuniform_scale(0.5, 0.6, 0.7);
        let expected_class = SdfClassIndex(17);
        let expected_material_index = MaterialIndex(4);
        let expected_object_uid = ObjectUid(7);
        let expected_ray_marching_scale = 5.0;
        
        let system_under_test = SdfInstance::new(expected_location, expected_ray_marching_scale, expected_class, Linkage::new(expected_object_uid, expected_material_index));

        let mut container = GpuReadySerializationBuffer::new(1, SdfInstance::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);

        let location_serialized = {
            let mut location_serialized = GpuReadySerializationBuffer::new(2, 3);
            serialize_matrix_3x4(&mut location_serialized, &expected_location);
            let inverted_location = expected_location.invert().unwrap();
            serialize_matrix_3x4(&mut location_serialized, &inverted_location);
            cast_slice::<u8, f32>(&location_serialized.backend()).to_vec()
        };
        let matrix_3x4_floats = MATRIX_FLOATS_COUNT - 4;
        
        let serialized: &[f32] = cast_slice(&container.backend());
        
        let mut values_checked = 0;
        assert_eq!(&serialized[values_checked..values_checked + matrix_3x4_floats], &location_serialized[0..matrix_3x4_floats]);
        values_checked += matrix_3x4_floats;
        assert_eq!(&serialized[values_checked..values_checked + matrix_3x4_floats], &location_serialized[matrix_3x4_floats..(matrix_3x4_floats) * 2]);
        values_checked += matrix_3x4_floats;

        assert_eq!(serialized[values_checked], expected_ray_marching_scale as f32);
        values_checked += 1;
        assert_eq!(serialized[values_checked], expected_class.as_f64() as f32);
        values_checked += 1;
        assert_eq!(serialized[values_checked].to_bits(), expected_material_index.0 as u32);
        values_checked += 1;
        assert_eq!(serialized[values_checked].to_bits(), expected_object_uid.0);
        values_checked += 1;
        
        assert_eq!(values_checked, SdfInstance::SERIALIZED_QUARTET_COUNT * ELEMENTS_IN_QUARTET);
    }
}