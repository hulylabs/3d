use std::ops::Add;
use crate::geometry::aabb::AABB;
use crate::geometry::transform::Transformation;
use crate::geometry::vertex::Vertex;
use crate::objects::utils::serialization_helpers::GpuFloatBufferFiller;
use crate::panic_if_failed;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MeshIndex(pub u32);
impl MeshIndex {
    #[must_use]
    pub(crate) const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct TriangleIndex(pub u32);
impl TriangleIndex {
    #[must_use]
    pub(crate) const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}
impl Add<usize> for TriangleIndex {
    type Output = TriangleIndex;
    fn add(self, rhs: usize) -> Self::Output {
        TriangleIndex {0: self.0 + rhs as u32}
    }
}

pub(crate) enum TriangleVertex
{
    A,
    B,
    C,

    Count,
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
        Self { a, b, c, in_kind_index, host_mesh_index }
    }

    pub(crate) fn bounding_box(&self) -> AABB {
        let result = AABB::from_triangle(
            &self.a.position(),
            &self.b.position(),
            &self.c.position(),
        );
        result.pad()
    }

    pub(crate) fn transform(&self, transformation: &Transformation) -> Triangle {
        Triangle {
            a: self.a.transform(transformation),
            b: self.b.transform(transformation),
            c: self.c.transform(transformation),
            in_kind_index: self.in_kind_index,
            host_mesh_index: self.host_mesh_index,
        }
    }

    const SERIALIZED_QUARTET_COUNT: usize = 6;
    pub(crate) const SERIALIZED_SIZE: usize = Triangle::SERIALIZED_QUARTET_COUNT * <[f32] as GpuFloatBufferFiller>::FLOAT_ALIGNMENT_SIZE;

    pub(crate) fn serialize_into(&self, container: &mut [f32]) {
        panic_if_failed!(container.len() >= Triangle::SERIALIZED_SIZE, "buffer size is too small");

        let mut index = 0;

        container.write_and_move_next(self.a.position().x,           &mut index);
        container.write_and_move_next(self.a.position().y,           &mut index);
        container.write_and_move_next(self.a.position().z,           &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.b.position().x,           &mut index);
        container.write_and_move_next(self.b.position().y,           &mut index);
        container.write_and_move_next(self.b.position().z,           &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.c.position().x,           &mut index);
        container.write_and_move_next(self.c.position().y,           &mut index);
        container.write_and_move_next(self.c.position().z,           &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.a.normal().x,             &mut index);
        container.write_and_move_next(self.a.normal().y,             &mut index);
        container.write_and_move_next(self.a.normal().z,             &mut index);
        container.pad_to_align(&mut index);

        container.write_and_move_next(self.b.normal().x,             &mut index);
        container.write_and_move_next(self.b.normal().y,             &mut index);
        container.write_and_move_next(self.b.normal().z,             &mut index);
        container.write_and_move_next(self.in_kind_index.as_f32(),   &mut index);

        container.write_and_move_next(self.c.normal().x,             &mut index);
        container.write_and_move_next(self.c.normal().y,             &mut index);
        container.write_and_move_next(self.c.normal().z,             &mut index);
        container.write_and_move_next(self.host_mesh_index.as_f32(), &mut index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::epsilon::DEFAULT_EPSILON;
    use crate::geometry::transform::Affine;
    use cgmath::{assert_abs_diff_eq, Rad, Transform};
    use std::f32::consts::PI;

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
        assert_eq!(actual_bounding_box.max(), Point::new( 0.2,  0.4,  0.6));
    }

    #[test]
    fn test_triangle_transform() {
        let a = Vertex::new(Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0));
        let b = Vertex::new(Point::new(0.0, 1.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let c = Vertex::new(Point::new(0.0, 0.0, 1.0), Vector::new(0.0, 0.0, 1.0));
        let expected_in_kind_index = TriangleIndex(0);
        let expected_host_mesh_index = MeshIndex(0);
        let system_under_test = Triangle::new(a, b, c, expected_in_kind_index, expected_host_mesh_index);

        let matrix = Affine::from_translation(Vector::unit_x()) * Affine::from_angle_z(Rad(PI / 2.0));

        let actual_triangle = system_under_test.transform(&Transformation::new(matrix));

        let expected_a = Vertex::new(Point::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let expected_b = Vertex::new(Point::new(0.0, 0.0, 0.0), Vector::new(-1.0, 0.0, 0.0));
        let expected_c = Vertex::new(Point::new(1.0, 0.0, 1.0), Vector::new(0.0, 0.0, 1.0));

        let epsilon = DEFAULT_EPSILON;
        assert_abs_diff_eq!(actual_triangle.a, expected_a, epsilon=epsilon);
        assert_abs_diff_eq!(actual_triangle.b, expected_b, epsilon=epsilon);
        assert_abs_diff_eq!(actual_triangle.c, expected_c, epsilon=epsilon);

        assert_eq!(actual_triangle.host_mesh_index, expected_host_mesh_index);
        assert_eq!(actual_triangle.in_kind_index, expected_in_kind_index);
    }

    #[test]
    fn test_triangle_serialization() {
        let a = Vertex::new(Point::new(1.0, 2.0, 3.0), Vector::new(10.0, 11.0, 12.0));
        let b = Vertex::new(Point::new(4.0, 5.0, 6.0), Vector::new(13.0, 14.0, 15.0));
        let c = Vertex::new(Point::new(7.0, 8.0, 9.0), Vector::new(16.0, 17.0, 18.0));
        let expected_in_kind_index = TriangleIndex(19);
        let expected_host_mesh_index = MeshIndex(20);
        let container_initial_filler = -7.0;
        let system_under_test = Triangle::new(a, b, c, expected_in_kind_index, expected_host_mesh_index);

        let mut container = vec![container_initial_filler; Triangle::SERIALIZED_SIZE + 1];
        system_under_test.serialize_into(&mut container);

        let expected = vec![
            a.position().x,
            a.position().y,
            a.position().z,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,

            b.position().x,
            b.position().y,
            b.position().z,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,

            c.position().x,
            c.position().y,
            c.position().z,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,

            a.normal().x,
            a.normal().y,
            a.normal().z,
            <[f32] as GpuFloatBufferFiller>::PAD_VALUE,

            b.normal().x,
            b.normal().y,
            b.normal().z,
            expected_in_kind_index.as_f32(),

            c.normal().x,
            c.normal().y,
            c.normal().z,
            expected_host_mesh_index.as_f32(),

            container_initial_filler,
        ];

        assert_eq!(container, expected);
    }
}