use crate::geometry::alias;

use crate::objects::common_properties::Linkage;
use crate::serialization::filler::{GpuFloatBufferFiller, floats_count};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use alias::Point;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SphereIndex(pub(crate) usize);
impl SphereIndex {
    #[must_use]
    pub(crate) const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}
impl From<usize> for SphereIndex {
    fn from(value: usize) -> Self {
        SphereIndex(value)
    }
}

pub(crate) struct Sphere {
    center: Point,
    radius: f64,
    links: Linkage<SphereIndex>,
}

impl Sphere {
    #[must_use]
    pub(crate) fn new(center: Point, radius: f64, links: Linkage<SphereIndex>) -> Self {
        assert!(radius > 0.0, "radius must be positive");
        Sphere { center, radius, links }
    }

    const SERIALIZED_QUARTET_COUNT: usize = 2;
}

impl SerializableForGpu for Sphere {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Sphere::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Sphere::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;
        container.write_and_move_next(self.center.x, &mut index);
        container.write_and_move_next(self.center.y, &mut index);
        container.write_and_move_next(self.center.z, &mut index);
        container.write_and_move_next(self.radius, &mut index);

        container.write_and_move_next(self.links.global_index().as_f64(), &mut index);
        container.write_and_move_next(self.links.in_kind_index().as_f64(), &mut index);
        container.write_and_move_next(self.links.material_index().as_f64(), &mut index);
        container.pad_to_align(&mut index);

        debug_assert_eq!(index, Sphere::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::common_properties::GlobalObjectIndex;
    use crate::objects::material_index::MaterialIndex;
    use cgmath::EuclideanSpace;

    #[test]
    fn test_origin() {
        let origin = Point::origin();
        assert_eq!(origin.x, 0.0);
        assert_eq!(origin.y, 0.0);
        assert_eq!(origin.z, 0.0);
    }

    const DUMMY_LINKS: Linkage<SphereIndex> = Linkage::new(GlobalObjectIndex(1), SphereIndex(1), MaterialIndex(1));

    #[test]
    #[should_panic(expected = "radius must be positive")]
    fn test_new_with_negative_radius() {
        let _system_under_test = Sphere::new(Point::origin(), -1.0, DUMMY_LINKS);
    }

    #[test]
    fn test_new_with_valid_radius() {
        let expected_center = Point::new(3.0, 4.0, 5.0);
        let expected_radius = 6.0;
        let expected_links = Linkage::new(GlobalObjectIndex(7), SphereIndex(9), MaterialIndex(8));

        let system_under_test = Sphere::new(expected_center, expected_radius, expected_links);

        assert_eq!(system_under_test.radius, expected_radius);
        assert_eq!(system_under_test.center, expected_center);
        assert_eq!(system_under_test.links, expected_links);
    }

    #[test]
    fn test_serialize_into() {
        let center = Point::new(1.0, 2.0, 3.0);
        let radius = 4.0;
        let expected_global_index = GlobalObjectIndex(4);
        let expected_local_index = SphereIndex(5);
        let expected_material_index = MaterialIndex(6);
        let system_under_test = Sphere::new(center, radius, Linkage::new(expected_global_index, expected_local_index, expected_material_index));
        let container_initial_filler = -7.0;

        let mut container = vec![container_initial_filler; Sphere::SERIALIZED_SIZE_FLOATS + 1];
        system_under_test.serialize_into(&mut container);

        assert_eq!(container[0], center.x as f32);
        assert_eq!(container[1], center.y as f32);
        assert_eq!(container[2], center.z as f32);
        assert_eq!(container[3], radius as f32);
        assert_eq!(container[4], expected_global_index.0 as f32);
        assert_eq!(container[5], expected_local_index.0 as f32);
        assert_eq!(container[6], expected_material_index.0 as f32);
        assert_eq!(container[7], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);

        assert_eq!(container[8], container_initial_filler);
    }

    #[test]
    #[should_panic(expected = "buffer size is too small")]
    fn test_serialize_into_with_small_buffer() {
        let system_under_test = Sphere::new(Point::origin(), 1.0, DUMMY_LINKS);

        let mut container = vec![0.0; Sphere::SERIALIZED_SIZE_FLOATS - 1];
        system_under_test.serialize_into(&mut container);
    }
}
