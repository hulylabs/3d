use crate::geometry::aabb::Aabb;
use crate::geometry::epsilon::DEFAULT_EPSILON;
use crate::geometry::vertex::Vertex;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use cgmath::AbsDiffEq;
use std::ops::Add;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MeshIndex(pub(crate) usize);
impl MeshIndex {
    #[must_use]
    pub(crate) const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}
impl From<usize> for MeshIndex {
    fn from(value: usize) -> Self {
        MeshIndex(value)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct TriangleIndex(pub(crate) usize);
impl TriangleIndex {
    #[must_use]
    pub(crate) const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}
impl Add<usize> for TriangleIndex {
    type Output = TriangleIndex;
    fn add(self, right: usize) -> Self::Output {
        TriangleIndex(self.0 + right)
    }
}

pub(crate) enum TriangleVertex {
    A,
    B,
    C,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct Triangle {
    a: Vertex,
    b: Vertex,
    c: Vertex,
    in_kind_index: TriangleIndex,
    host_mesh_index: MeshIndex,
}

impl Triangle {
    #[must_use]
    pub(crate) fn new(a: Vertex, b: Vertex, c: Vertex, in_kind_index: TriangleIndex, host_mesh_index: MeshIndex) -> Self {
        Self {
            a,
            b,
            c,
            in_kind_index,
            host_mesh_index,
        }
    }

    #[must_use]
    pub(crate) fn bounding_box(&self) -> Aabb {
        let result = Aabb::from_triangle(self.a.position(), self.b.position(), self.c.position());
        result.pad()
    }
}

impl AbsDiffEq for Triangle {
    type Epsilon = f64;

    #[must_use]
    fn default_epsilon() -> Self::Epsilon {
        DEFAULT_EPSILON
    }

    #[must_use]
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
           Vertex::abs_diff_eq(&self.a, &other.a, epsilon)
        && Vertex::abs_diff_eq(&self.b, &other.b, epsilon)
        && Vertex::abs_diff_eq(&self.c, &other.c, epsilon)
        && self.in_kind_index == other.in_kind_index
        && self.host_mesh_index == other.host_mesh_index
    }
}

impl SerializableForGpu for Triangle {
    const SERIALIZED_QUARTET_COUNT: usize = 6;

    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        container.write_padded_quartet_f64(self.a.position().x, self.a.position().y, self.a.position().z);
        container.write_padded_quartet_f64(self.b.position().x, self.b.position().y, self.b.position().z);
        container.write_padded_quartet_f64(self.c.position().x, self.c.position().y, self.c.position().z);

        container.write_padded_quartet_f64(self.a.normal().x, self.a.normal().y, self.a.normal().z);
        container.write_quartet_f64       (self.b.normal().x, self.b.normal().y, self.b.normal().z, self.in_kind_index.as_f64());
        container.write_quartet_f64       (self.c.normal().x, self.c.normal().y, self.c.normal().z, self.host_mesh_index.as_f64());

        debug_assert!(container.object_fully_written());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;
    use bytemuck::cast_slice;

    #[test]
    fn test_triangle_creation() {
        let a = Vertex::new(Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0));
        let b = Vertex::new(Point::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let c = Vertex::new(Point::new(0.0, 1.0, 0.0), Vector::new(0.0, 0.0, 1.0));

        let in_kind_index = TriangleIndex(0);
        let host_mesh_index = MeshIndex(0);

        let system_under_test = Triangle::new(a, b, c, in_kind_index, host_mesh_index);

        assert_eq!(system_under_test.a, a);
        assert_eq!(system_under_test.b, b);
        assert_eq!(system_under_test.c, c);

        assert_eq!(system_under_test.host_mesh_index, host_mesh_index);
        assert_eq!(system_under_test.in_kind_index, in_kind_index);
    }

    #[test]
    fn test_triangle_bounding_box() {
        let a = Vertex::new(Point::new(-9.0, 0.3, 0.5), Vector::new(1.0, 0.0, 0.0));
        let b = Vertex::new(Point::new(0.1, -8.0, 0.6), Vector::new(0.0, 2.0, 0.0));
        let c = Vertex::new(Point::new(0.2, 0.4, -7.0), Vector::new(0.0, 0.0, 3.0));
        let system_under_test = Triangle::new(a, b, c, TriangleIndex(0), MeshIndex(0));

        let actual_bounding_box = system_under_test.bounding_box();

        assert_eq!(actual_bounding_box.min(), Point::new(-9.0, -8.0, -7.0));
        assert_eq!(actual_bounding_box.max(), Point::new(0.2, 0.4, 0.6));
    }

    #[test]
    fn test_triangle_serialization() {
        let a = Vertex::new(Point::new(1.0, 2.0, 3.0), Vector::new(10.0, 11.0, 12.0));
        let b = Vertex::new(Point::new(4.0, 5.0, 6.0), Vector::new(13.0, 14.0, 15.0));
        let c = Vertex::new(Point::new(7.0, 8.0, 9.0), Vector::new(16.0, 17.0, 18.0));
        let expected_in_kind_index = TriangleIndex(19);
        let expected_host_mesh_index = MeshIndex(20);
        let system_under_test = Triangle::new(a, b, c, expected_in_kind_index, expected_host_mesh_index);

        let mut actual_state = GpuReadySerializationBuffer::new(1, Triangle::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut actual_state);

        let expected: Vec<f32> = vec![
            a.position().x as f32,
            a.position().y as f32,
            a.position().z as f32,
            DEFAULT_PAD_VALUE,
            b.position().x as f32,
            b.position().y as f32,
            b.position().z as f32,
            DEFAULT_PAD_VALUE,
            c.position().x as f32,
            c.position().y as f32,
            c.position().z as f32,
            DEFAULT_PAD_VALUE,
            a.normal().x as f32,
            a.normal().y as f32,
            a.normal().z as f32,
            DEFAULT_PAD_VALUE,
            b.normal().x as f32,
            b.normal().y as f32,
            b.normal().z as f32,
            expected_in_kind_index.as_f64() as f32,
            c.normal().x as f32,
            c.normal().y as f32,
            c.normal().z as f32,
            expected_host_mesh_index.as_f64() as f32,
        ];

        assert_eq!(actual_state.backend(), cast_slice(&expected));
    }
}
