use crate::geometry::alias;

use crate::objects::common_properties::Linkage;
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use alias::Point;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(crate) struct Sphere {
    center: Point,
    radius: f64,
    links: Linkage,
}

impl Sphere {
    #[must_use]
    pub(crate) fn new(center: Point, radius: f64, links: Linkage) -> Self {
        assert!(radius > 0.0, "radius must be positive");
        Sphere { center, radius, links }
    }
}

impl SerializableForGpu for Sphere {
    const SERIALIZED_QUARTET_COUNT: usize = 2;

    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        container.write_quartet_f64(self.center.x, self.center.y, self.center.z, self.radius);
        container.write(|writer| {
            writer
                .write_integer(self.links.uid().0)
                .write_float(self.links.material_index().as_f64() as f32);
        });

        debug_assert!(container.object_fully_written());
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use super::*;
    use crate::objects::common_properties::ObjectUid;
    use crate::objects::material_index::MaterialIndex;
    use cgmath::EuclideanSpace;
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;

    #[test]
    fn test_origin() {
        let origin = Point::origin();
        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
        assert_eq!(origin.z, 0.0);
    }

    const DUMMY_LINKS: Linkage = Linkage::new(ObjectUid(1), MaterialIndex(1));

    #[test]
    #[should_panic(expected = "radius must be positive")]
    fn test_new_with_negative_radius() {
        let _system_under_test = Sphere::new(Point::origin(), -1.0, DUMMY_LINKS);
    }

    #[test]
    fn test_new_with_valid_radius() {
        let expected_center = Point::new(3.0, 4.0, 5.0);
        let expected_radius = 6.0;
        let expected_links = Linkage::new(ObjectUid(7), MaterialIndex(8));

        let system_under_test = Sphere::new(expected_center, expected_radius, expected_links);

        assert_eq!(system_under_test.radius, expected_radius);
        assert_eq!(system_under_test.center, expected_center);
        assert_eq!(system_under_test.links, expected_links);
    }

    #[test]
    fn test_serialize_into() {
        let center = Point::new(1.0, 2.0, 3.0);
        let radius = 4.0;
        let expected_uid = ObjectUid(4);
        let expected_material_index = MaterialIndex(6);
        let system_under_test = Sphere::new(center, radius, Linkage::new(expected_uid, expected_material_index));

        let mut actual_state = GpuReadySerializationBuffer::new(1, Sphere::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut actual_state);
        let serialized: &[f32] = cast_slice(&actual_state.backend());

        assert_eq!(serialized[0], center.x as f32);
        assert_eq!(serialized[1], center.y as f32);
        assert_eq!(serialized[2], center.z as f32);
        assert_eq!(serialized[3], radius as f32);
        assert_eq!(serialized[4].to_bits(), expected_uid.0);
        assert_eq!(serialized[5], expected_material_index.0 as f32);
        assert_eq!(serialized[6], DEFAULT_PAD_VALUE);
        assert_eq!(serialized[7], DEFAULT_PAD_VALUE);
    }

    #[test]
    #[should_panic(expected = "buffer overflow")]
    fn test_serialize_into_with_small_buffer() {
        let system_under_test = Sphere::new(Point::origin(), 1.0, DUMMY_LINKS);

        let mut container = GpuReadySerializationBuffer::new(1, Sphere::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);
        system_under_test.serialize_into(&mut container);
    }
}
