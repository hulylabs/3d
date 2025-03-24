use crate::geometry::aabb::AABB;
use crate::geometry::alias;
use cgmath::EuclideanSpace;
use cgmath::InnerSpace;


use crate::objects::utils::common_properties::Linkage;
use crate::objects::utils::serialization_helpers::GpuFloatBufferFiller;
use crate::panic_if_failed;
use alias::Point;
use alias::Vector;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct QuadrilateralIndex(pub u32);
impl QuadrilateralIndex {
    #[must_use]
    pub(crate) const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

pub(crate) struct Quadrilateral {
    origin: Point,
    local_x: Vector,
    local_y: Vector,
    links: Linkage<QuadrilateralIndex>,
}

impl Quadrilateral {
    #[must_use]
    pub const fn new(
        origin: Point,
        local_x: Vector,
        local_y: Vector,
        links: Linkage<QuadrilateralIndex>,
    ) -> Self {
        Quadrilateral {
            origin,
            local_x,
            local_y,
            links,
        }
    }

    #[must_use]
    pub(crate) fn bounding_box(&self) -> AABB {
        let result = AABB::from_segment(
            self.origin,
            self.origin + self.local_x + self.local_y,
        );
        result.pad()
    }

    const SERIALIZED_QUARTET_COUNT: usize = 5;
    pub(crate) const SERIALIZED_SIZE: usize = Quadrilateral::SERIALIZED_QUARTET_COUNT * <[f32] as GpuFloatBufferFiller>::FLOAT_ALIGNMENT_SIZE;

    pub(crate) fn serialize_into(&self, container: &mut [f32]) {
        panic_if_failed!(container.len() >= Quadrilateral::SERIALIZED_SIZE, "buffer size is too small");

        let orth = self.local_x.cross(self.local_y);
        let orth_sqr = orth.dot(orth);
        let normal = orth / orth_sqr.sqrt();
        let distance_to_origin = normal.dot(self.origin.to_vec()); // d from plane's equation ax+by+cz+d = 0, where (a,b,c) is normal
        let w = orth / orth_sqr; //TODO: geometry meaning?

        let mut index = 0;

        container.write_and_move_next(self.origin.x,                        &mut index);
        container.write_and_move_next(self.origin.y,                        &mut index);
        container.write_and_move_next(self.origin.z,                        &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.local_x.x,                       &mut index);
        container.write_and_move_next(self.local_x.y,                       &mut index);
        container.write_and_move_next(self.local_x.z,                       &mut index);
        container.write_and_move_next(self.links.in_kind_index().as_f32(),  &mut index);

        container.write_and_move_next(self.local_y.x,                       &mut index);
        container.write_and_move_next(self.local_y.y,                       &mut index);
        container.write_and_move_next(self.local_y.z,                       &mut index);
        container.write_and_move_next(self.links.global_index().as_f32(),   &mut index);

        container.write_and_move_next(normal.x,                             &mut index);
        container.write_and_move_next(normal.y,                             &mut index);
        container.write_and_move_next(normal.z,                             &mut index);
        container.write_and_move_next(distance_to_origin,                   &mut index);

        container.write_and_move_next(w.x,                                  &mut index);
        container.write_and_move_next(w.y,                                  &mut index);
        container.write_and_move_next(w.z,                                  &mut index);
        container.write_and_move_next(self.links.material_index().as_f32(), &mut index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::utils::common_properties::{GlobalObjectIndex, Linkage, MaterialIndex};

    const DUMMY_LINKS: Linkage<QuadrilateralIndex> = Linkage::new(GlobalObjectIndex(1), QuadrilateralIndex(1), MaterialIndex(1));

    #[test]
    fn test_new_quadrilateral() {
        let expected_origin = Point::new(1.0, 2.0, 3.0);
        let expected_local_x = Vector::new(4.0, 5.0, 6.0);
        let expected_local_y = Vector::new(7.0, 8.0, 9.0);
        let expected_links = Linkage::new(GlobalObjectIndex(10), QuadrilateralIndex(11), MaterialIndex(12));

        let system_under_test = Quadrilateral::new(expected_origin, expected_local_x, expected_local_y, expected_links);

        assert_eq!(system_under_test.origin, expected_origin);
        assert_eq!(system_under_test.local_x, expected_local_x);
        assert_eq!(system_under_test.local_y, expected_local_y);
        assert_eq!(system_under_test.links, expected_links);
    }

    #[test]
    fn test_bounding_box() {
        let origin = Point::new(-1.0, -2.0, -3.0);
        let local_x = Vector::new(4.0, -5.0, 6.0);
        let local_y = Vector::new(7.0, -8.0, 9.0);
        let system_under_test = Quadrilateral::new(origin, local_x, local_y, DUMMY_LINKS);

        let bounding_box = system_under_test.bounding_box();

        assert_eq!(bounding_box.min(), Point::new(-1.0, -15.0, -3.0));
        assert_eq!(bounding_box.max(), Point::new(10.0, -2.0, 12.0));
    }

    #[test]
    fn test_serialize_into() {
        let origin = Point::new(1.0, 2.0, 3.0);
        let local_x = Vector::new(0.0, 2.0, 0.0);
        let local_y = Vector::new(2.0, 0.0, 0.0);
        let expected_global_index = GlobalObjectIndex(11);
        let expected_local_index = QuadrilateralIndex(13);
        let expected_material_index = MaterialIndex(17);
        let system_under_test = Quadrilateral::new(origin, local_x, local_y, Linkage::new(expected_global_index, expected_local_index, expected_material_index));
        let buffer_initial_filler = -1.0;

        let mut container = vec![buffer_initial_filler; Quadrilateral::SERIALIZED_SIZE + 1];
        system_under_test.serialize_into(&mut container);

        assert_eq!(container[0],  origin.x);
        assert_eq!(container[1],  origin.y);
        assert_eq!(container[2],  origin.z);
        assert_eq!(container[3],  <[f32] as GpuFloatBufferFiller>::PAD_VALUE);

        assert_eq!(container[4],  local_x.x);
        assert_eq!(container[5],  local_x.y);
        assert_eq!(container[6],  local_x.z);
        assert_eq!(container[7],  expected_local_index.0 as f32);

        assert_eq!(container[8],  local_y.x);
        assert_eq!(container[9],  local_y.y);
        assert_eq!(container[10], local_y.z);
        assert_eq!(container[11], expected_global_index.0 as f32);

        assert_eq!(container[12], 0.0);
        assert_eq!(container[13], 0.0);
        assert_eq!(container[14], -1.0);
        assert_eq!(container[15], -3.0);
        assert_eq!(container[16], 0.0);
        assert_eq!(container[17], 0.0);
        assert_eq!(container[18], -4.0 / 16.0);
        assert_eq!(container[19], expected_material_index.0 as f32);
    }

    #[test]
    #[should_panic(expected = "buffer size is too small")]
    fn test_serialize_into_with_small_buffer() {
        let system_under_test = Quadrilateral::new(Point::origin(), Vector::unit_x(), Vector::unit_y(), DUMMY_LINKS);
        let mut container = vec![0.0; Quadrilateral::SERIALIZED_SIZE - 1];

        system_under_test.serialize_into(&mut container);
    }
}