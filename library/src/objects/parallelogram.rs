use crate::geometry::alias;
use cgmath::EuclideanSpace;
use cgmath::InnerSpace;

use crate::objects::common_properties::Linkage;
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use alias::Point;
use alias::Vector;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

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
}

impl SerializableForGpu for Parallelogram {
    const SERIALIZED_QUARTET_COUNT: usize = 5;

    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        let orth = self.local_x.cross(self.local_y);
        let orth_square = orth.dot(orth);
        let normal = orth / orth_square.sqrt();
        let distance_to_origin = normal.dot(self.origin.to_vec()); // d from plane's equation ax+by+cz+d = 0, where (a,b,c) is normal
        let w = orth / orth_square; //TODO: geometry meaning?

        container.write_padded_quartet_f64(
            self.origin.x,
            self.origin.y,
            self.origin.z,
        );

        container.write_quartet_f64(
            self.local_x.x,
            self.local_x.y,
            self.local_x.z,
            self.links.in_kind_index().as_f64(),
        );

        container.write_quartet_f64(
            self.local_y.x,
            self.local_y.y,
            self.local_y.z,
            self.links.global_index().as_f64(),
        );

        container.write_quartet_f64(
            normal.x,
            normal.y,
            normal.z,
            distance_to_origin,
        );

        container.write_quartet_f64(
            w.x,
            w.y,
            w.z,
            self.links.material_index().as_f64(),
        );

        debug_assert!(container.object_fully_written());
    }
}

#[cfg(test)]
mod tests {
    use bytemuck::cast_slice;
    use super::*;
    use crate::objects::common_properties::GlobalObjectIndex;
    use crate::objects::material_index::MaterialIndex;
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;

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

        let mut container = GpuReadySerializationBuffer::new(1, Parallelogram::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);

        let serialized: &[f32] = cast_slice(&container.backend());

        assert_eq!(serialized[0 ], origin.x as f32);
        assert_eq!(serialized[1 ], origin.y as f32);
        assert_eq!(serialized[2 ], origin.z as f32);
        assert_eq!(serialized[3 ], DEFAULT_PAD_VALUE);

        assert_eq!(serialized[4 ], local_x.x as f32);
        assert_eq!(serialized[5 ], local_x.y as f32);
        assert_eq!(serialized[6 ], local_x.z as f32);
        assert_eq!(serialized[7 ], expected_local_index.0 as f32);

        assert_eq!(serialized[8 ], local_y.x as f32);
        assert_eq!(serialized[9 ], local_y.y as f32);
        assert_eq!(serialized[10], local_y.z as f32);
        assert_eq!(serialized[11], expected_global_index.0 as f32);

        assert_eq!(serialized[12], 0.0);
        assert_eq!(serialized[13], 0.0);
        assert_eq!(serialized[14], -1.0);
        assert_eq!(serialized[15], -3.0);
        assert_eq!(serialized[16], 0.0);
        assert_eq!(serialized[17], 0.0);
        assert_eq!(serialized[18], -4.0 / 16.0);
        assert_eq!(serialized[19], expected_material_index.0 as f32);
    }
}
