use crate::geometry::aabb::AABB;
use crate::geometry::alias;

use crate::objects::utils::common_properties::Linkage;
use crate::objects::utils::serialization_helpers::GpuFloatBufferFiller;
use crate::panic_if_failed;
use alias::Point;
use alias::Vector;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct SphereIndex(pub u32);
impl SphereIndex {
    #[must_use]
    pub(crate) const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

pub(crate) struct Sphere {
    center: Point,
    radius: f32,
    links: Linkage<SphereIndex>,
}

impl Sphere {
    #[must_use]
    pub(crate) fn new(
        center: Point,
        radius: f32,
        links: Linkage<SphereIndex>,
    ) -> Self {
        if radius <= 0.0 {
            panic!("radius must be positive");
        }
        Sphere {
            center,
            radius,
            links,
        }
    }

    pub(crate) fn bounding_box(&self) -> AABB {
        let radius = Vector::new(self.radius, self.radius, self.radius);
        AABB::from_segment(
            self.center - radius,
            self.center + radius,
        )
    }

    const SERIALIZED_QUARTET_COUNT: usize = 2;
    pub(crate) const SERIALIZED_SIZE: usize = Sphere::SERIALIZED_QUARTET_COUNT * <[f32] as GpuFloatBufferFiller>::FLOAT_ALIGNMENT_SIZE;

    pub(crate) fn serialize_into(&self, container: &mut [f32]) {
        panic_if_failed!(container.len() >= Sphere::SERIALIZED_SIZE, "buffer size is too small");

        let mut index = 0;
        container.write_and_move_next(self.center.x,                          &mut index);
        container.write_and_move_next(self.center.y,                          &mut index);
        container.write_and_move_next(self.center.z,                          &mut index);
        container.write_and_move_next(self.radius,                            &mut index);

        container.write_and_move_next(self.links.global_index().as_f32(),     &mut index);
        container.write_and_move_next(self.links.in_kind_index().as_f32(),    &mut index);
        container.write_and_move_next(self.links.material_index().as_f32(),   &mut index);
        container.pad_to_align(&mut index);
    }
}

#[cfg(test)]
mod tests {
    use cgmath::EuclideanSpace;

    use super::*;
    use crate::objects::utils::common_properties::{GlobalObjectIndex, Linkage, MaterialIndex};

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

        let system_under_test
            = Sphere::new(
            expected_center,
            expected_radius,
            expected_links);

        assert_eq!(system_under_test.radius, expected_radius);
        assert_eq!(system_under_test.center, expected_center);
        assert_eq!(system_under_test.links, expected_links);
    }

    #[test]
    fn test_bounding_box() {
        let center = Point::new(1.0, 2.0, 3.0);
        let expected_radius = 6.0;
        let system_under_test = Sphere::new(center, expected_radius, DUMMY_LINKS);

        let bounding_box = system_under_test.bounding_box();

        assert_eq!(bounding_box.min(), Point::new(-5.0, -4.0, -3.0));
        assert_eq!(bounding_box.max(), Point::new( 7.0,  8.0,  9.0));
    }

    #[test]
    fn test_serialize_into() {
        let center = Point::new(1.0, 2.0, 3.0);
        let radius = 4.0;
        let expected_global_index = GlobalObjectIndex(4);
        let expected_local_index = SphereIndex(5);
        let expected_material_index = MaterialIndex(6);
        let system_under_test
            = Sphere::new(
            center,
            radius,
            Linkage::new(expected_global_index, expected_local_index, expected_material_index));
        let container_initial_filler = -1.0;

        let mut container = vec![container_initial_filler; Sphere::SERIALIZED_SIZE + 1];
        system_under_test.serialize_into(&mut container);

        assert_eq!(container[0], center.x);
        assert_eq!(container[1], center.y);
        assert_eq!(container[2], center.z);
        assert_eq!(container[3], radius);
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

        let mut container = vec![0.0; Sphere::SERIALIZED_SIZE - 1];
        system_under_test.serialize_into(&mut container);
    }
}