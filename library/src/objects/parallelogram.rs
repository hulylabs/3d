use crate::geometry::alias;
use cgmath::EuclideanSpace;
use cgmath::InnerSpace;

use crate::objects::common_properties::Linkage;
use crate::serialization::filler::{GpuFloatBufferFiller, floats_count};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use alias::Point;
use alias::Vector;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ParallelogramIndex(pub(crate) usize);
impl ParallelogramIndex {
    #[must_use]
    pub(crate) const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}
impl From<usize> for ParallelogramIndex {
    #[must_use]
    fn from(value: usize) -> Self {
        ParallelogramIndex(value)
    }
}

pub(crate) struct Parallelogram {
    origin: Point,
    local_x: Vector,
    local_y: Vector,
    links: Linkage<ParallelogramIndex>,
}

impl Parallelogram {
    #[must_use]
    pub const fn new(origin: Point, local_x: Vector, local_y: Vector, links: Linkage<ParallelogramIndex>) -> Self {
        Parallelogram { origin, local_x, local_y, links }
    }

    const SERIALIZED_QUARTET_COUNT: usize = 5;
}

impl SerializableForGpu for Parallelogram {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Parallelogram::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Parallelogram::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let orth = self.local_x.cross(self.local_y);
        let orth_square = orth.dot(orth);
        let normal = orth / orth_square.sqrt();
        let distance_to_origin = normal.dot(self.origin.to_vec()); // d from plane's equation ax+by+cz+d = 0, where (a,b,c) is normal
        let w = orth / orth_square; //TODO: geometry meaning?

        let mut index = 0;

        container.write_and_move_next(self.origin.x, &mut index);
        container.write_and_move_next(self.origin.y, &mut index);
        container.write_and_move_next(self.origin.z, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.local_x.x, &mut index);
        container.write_and_move_next(self.local_x.y, &mut index);
        container.write_and_move_next(self.local_x.z, &mut index);
        container.write_and_move_next(self.links.in_kind_index().as_f64(), &mut index);

        container.write_and_move_next(self.local_y.x, &mut index);
        container.write_and_move_next(self.local_y.y, &mut index);
        container.write_and_move_next(self.local_y.z, &mut index);
        container.write_and_move_next(self.links.global_index().as_f64(), &mut index);

        container.write_and_move_next(normal.x, &mut index);
        container.write_and_move_next(normal.y, &mut index);
        container.write_and_move_next(normal.z, &mut index);
        container.write_and_move_next(distance_to_origin, &mut index);

        container.write_and_move_next(w.x, &mut index);
        container.write_and_move_next(w.y, &mut index);
        container.write_and_move_next(w.z, &mut index);
        container.write_and_move_next(self.links.material_index().as_f64(), &mut index);

        debug_assert_eq!(index, Parallelogram::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::common_properties::GlobalObjectIndex;
    use crate::objects::material_index::MaterialIndex;

    const DUMMY_LINKS: Linkage<ParallelogramIndex> = Linkage::new(GlobalObjectIndex(1), ParallelogramIndex(1), MaterialIndex(1));

    #[test]
    fn test_new_quadrilateral() {
        let expected_origin = Point::new(1.0, 2.0, 3.0);
        let expected_local_x = Vector::new(4.0, 5.0, 6.0);
        let expected_local_y = Vector::new(7.0, 8.0, 9.0);
        let expected_links = Linkage::new(GlobalObjectIndex(10), ParallelogramIndex(11), MaterialIndex(12));

        let system_under_test = Parallelogram::new(expected_origin, expected_local_x, expected_local_y, expected_links);

        assert_eq!(system_under_test.origin, expected_origin);
        assert_eq!(system_under_test.local_x, expected_local_x);
        assert_eq!(system_under_test.local_y, expected_local_y);
        assert_eq!(system_under_test.links, expected_links);
    }

    #[test]
    fn test_serialize_into() {
        let origin = Point::new(1.0, 2.0, 3.0);
        let local_x = Vector::new(0.0, 2.0, 0.0);
        let local_y = Vector::new(2.0, 0.0, 0.0);
        let expected_global_index = GlobalObjectIndex(11);
        let expected_local_index = ParallelogramIndex(13);
        let expected_material_index = MaterialIndex(17);
        let system_under_test = Parallelogram::new(origin, local_x, local_y, Linkage::new(expected_global_index, expected_local_index, expected_material_index));
        let buffer_initial_filler = -7.0;

        let mut container = vec![buffer_initial_filler; Parallelogram::SERIALIZED_SIZE_FLOATS + 1];
        system_under_test.serialize_into(&mut container);

        assert_eq!(container[0 ], origin.x as f32);
        assert_eq!(container[1 ], origin.y as f32);
        assert_eq!(container[2 ], origin.z as f32);
        assert_eq!(container[3 ], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);

        assert_eq!(container[4 ], local_x.x as f32);
        assert_eq!(container[5 ], local_x.y as f32);
        assert_eq!(container[6 ], local_x.z as f32);
        assert_eq!(container[7 ], expected_local_index.0 as f32);

        assert_eq!(container[8 ], local_y.x as f32);
        assert_eq!(container[9 ], local_y.y as f32);
        assert_eq!(container[10], local_y.z as f32);
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
        let system_under_test = Parallelogram::new(Point::origin(), Vector::unit_x(), Vector::unit_y(), DUMMY_LINKS);
        let mut container = vec![0.0; Parallelogram::SERIALIZED_SIZE_FLOATS - 1];

        system_under_test.serialize_into(&mut container);
    }
}
