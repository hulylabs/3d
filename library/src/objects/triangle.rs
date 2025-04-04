use crate::geometry::aabb::Aabb;
use crate::geometry::epsilon::DEFAULT_EPSILON;
use crate::geometry::vertex::Vertex;
use crate::serialization::helpers::{GpuFloatBufferFiller, floats_count};
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

    const SERIALIZED_QUARTET_COUNT: usize = 6;
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
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Triangle::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Triangle::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;

        container.write_and_move_next(self.a.position().x, &mut index);
        container.write_and_move_next(self.a.position().y, &mut index);
        container.write_and_move_next(self.a.position().z, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.b.position().x, &mut index);
        container.write_and_move_next(self.b.position().y, &mut index);
        container.write_and_move_next(self.b.position().z, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.c.position().x, &mut index);
        container.write_and_move_next(self.c.position().y, &mut index);
        container.write_and_move_next(self.c.position().z, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.a.normal().x, &mut index);
        container.write_and_move_next(self.a.normal().y, &mut index);
        container.write_and_move_next(self.a.normal().z, &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.b.normal().x, &mut index);
        container.write_and_move_next(self.b.normal().y, &mut index);
        container.write_and_move_next(self.b.normal().z, &mut index);
        container.write_and_move_next(self.in_kind_index.as_f64(), &mut index);

        container.write_and_move_next(self.c.normal().x, &mut index);
        container.write_and_move_next(self.c.normal().y, &mut index);
        container.write_and_move_next(self.c.normal().z, &mut index);
        container.write_and_move_next(self.host_mesh_index.as_f64(), &mut index);

        assert_eq!(index, Triangle::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};

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

    // #[test]
    // fn test_triangle_transform() {
    //     let a = Vertex::new(Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0));
    //     let b = Vertex::new(Point::new(0.0, 1.0, 0.0), Vector::new(0.0, 1.0, 0.0));
    //     let c = Vertex::new(Point::new(0.0, 0.0, 1.0), Vector::new(0.0, 0.0, 1.0));
    //     let expected_in_kind_index = TriangleIndex(0);
    //     let expected_host_mesh_index = MeshIndex(0);
    //     let system_under_test = Triangle::new(a, b, c, expected_in_kind_index, expected_host_mesh_index);
    //
    //     let matrix = Affine::from_translation(Vector::unit_x()) * Affine::from_angle_z(Rad(PI / 2.0));
    //
    //     let actual_triangle = system_under_test.transform(&Transformation::new(matrix));
    //
    //     let expected_a = Vertex::new(Point::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
    //     let expected_b = Vertex::new(Point::new(0.0, 0.0, 0.0), Vector::new(-1.0, 0.0, 0.0));
    //     let expected_c = Vertex::new(Point::new(1.0, 0.0, 1.0), Vector::new(0.0, 0.0, 1.0));
    //
    //     let epsilon = DEFAULT_EPSILON;
    //     assert_abs_diff_eq!(actual_triangle.a, expected_a, epsilon=epsilon);
    //     assert_abs_diff_eq!(actual_triangle.b, expected_b, epsilon=epsilon);
    //     assert_abs_diff_eq!(actual_triangle.c, expected_c, epsilon=epsilon);
    //
    //     assert_eq!(actual_triangle.host_mesh_index, expected_host_mesh_index);
    //     assert_eq!(actual_triangle.in_kind_index, expected_in_kind_index);
    // }

    #[test]
    fn test_triangle_serialization() {
        let a = Vertex::new(Point::new(1.0, 2.0, 3.0), Vector::new(10.0, 11.0, 12.0));
        let b = Vertex::new(Point::new(4.0, 5.0, 6.0), Vector::new(13.0, 14.0, 15.0));
        let c = Vertex::new(Point::new(7.0, 8.0, 9.0), Vector::new(16.0, 17.0, 18.0));
        let expected_in_kind_index = TriangleIndex(19);
        let expected_host_mesh_index = MeshIndex(20);
        let container_initial_filler = -7.0;
        let system_under_test = Triangle::new(a, b, c, expected_in_kind_index, expected_host_mesh_index);

        let mut container = vec![container_initial_filler; Triangle::SERIALIZED_SIZE_FLOATS + 1];
        system_under_test.serialize_into(&mut container);

        let expected: Vec<f32> = vec![
            a.position().x as f32,
            a.position().y as f32,
            a.position().z as f32,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,
            b.position().x as f32,
            b.position().y as f32,
            b.position().z as f32,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,
            c.position().x as f32,
            c.position().y as f32,
            c.position().z as f32,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,
            a.normal().x as f32,
            a.normal().y as f32,
            a.normal().z as f32,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,
            b.normal().x as f32,
            b.normal().y as f32,
            b.normal().z as f32,
            expected_in_kind_index.as_f64() as f32,
            c.normal().x as f32,
            c.normal().y as f32,
            c.normal().z as f32,
            expected_host_mesh_index.as_f64() as f32,
            container_initial_filler,
        ];

        assert_eq!(container, expected);
    }
}
