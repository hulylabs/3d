use crate::geometry::aabb::Aabb;
use crate::geometry::epsilon::DEFAULT_EPSILON_F64;
use crate::geometry::vertex::Vertex;
use crate::objects::common_properties::Linkage;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
use crate::utils::object_uid::ObjectUid;
use cgmath::AbsDiffEq;
use std::ops::Add;
use crate::material::material_index::MaterialIndex;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct TriangleIndex(pub(crate) usize);

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
    links: Linkage,
}

impl Triangle {
    #[must_use]
    pub(crate) fn new(a: Vertex, b: Vertex, c: Vertex, links: Linkage,) -> Self {
        Self {
            a,
            b,
            c,
            links,
        }
    }

    #[must_use]
    pub(crate) fn bounding_box(&self) -> Aabb {
        let result = Aabb::from_triangle(self.a.position(), self.b.position(), self.c.position());
        result.pad()
    }

    #[must_use]
    pub(crate) fn host(&self) -> ObjectUid {
        self.links.uid()
    }

    pub(crate) fn set_material(&mut self, new_material: MaterialIndex) {
        self.links.set_material_index(new_material);
    }
}

impl AbsDiffEq for Triangle {
    type Epsilon = f64;

    #[must_use]
    fn default_epsilon() -> Self::Epsilon {
        DEFAULT_EPSILON_F64
    }

    #[must_use]
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
           Vertex::abs_diff_eq(&self.a, &other.a, epsilon)
        && Vertex::abs_diff_eq(&self.b, &other.b, epsilon)
        && Vertex::abs_diff_eq(&self.c, &other.c, epsilon)
        && self.links == other.links
    }
}

impl GpuSerializationSize for Triangle {
    const SERIALIZED_QUARTET_COUNT: usize = 6;
}

impl GpuSerializable for Triangle {
    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        container.write_padded_quartet_f64(self.a.position().x, self.a.position().y, self.a.position().z);
        container.write_padded_quartet_f64(self.b.position().x, self.b.position().y, self.b.position().z);
        container.write_padded_quartet_f64(self.c.position().x, self.c.position().y, self.c.position().z);

        container.write_padded_quartet_f64(self.a.normal().x, self.a.normal().y, self.a.normal().z);

        container.write_quartet(|writer|{
            writer
                .write_float_64(self.b.normal().x)
                .write_float_64(self.b.normal().y)
                .write_float_64(self.b.normal().z)
                .write_unsigned(self.links.uid().0);
        });

        container.write_quartet(|writer|{
            writer
                .write_float_64(self.c.normal().x)
                .write_float_64(self.c.normal().y)
                .write_float_64(self.c.normal().z)
                .write_unsigned(self.links.material_index().0 as u32);
        });

        debug_assert!(container.object_fully_written());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::objects::common_properties::ObjectUid;
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;
    use bytemuck::cast_slice;

    #[test]
    fn test_triangle_creation() {
        let a = Vertex::new(Point::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0));
        let b = Vertex::new(Point::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0));
        let c = Vertex::new(Point::new(0.0, 1.0, 0.0), Vector::new(0.0, 0.0, 1.0));

        let expected_linkage = Linkage::new(ObjectUid(3), MaterialIndex(1));

        let system_under_test = Triangle::new(a, b, c, expected_linkage,);

        assert_eq!(system_under_test.a, a);
        assert_eq!(system_under_test.b, b);
        assert_eq!(system_under_test.c, c);

        assert_eq!(system_under_test.links, expected_linkage);
    }

    #[test]
    fn test_triangle_bounding_box() {
        let a = Vertex::new(Point::new(-9.0, 0.3, 0.5), Vector::new(1.0, 0.0, 0.0));
        let b = Vertex::new(Point::new(0.1, -8.0, 0.6), Vector::new(0.0, 2.0, 0.0));
        let c = Vertex::new(Point::new(0.2, 0.4, -7.0), Vector::new(0.0, 0.0, 3.0));
        let system_under_test = Triangle::new(a, b, c, Linkage::new(ObjectUid(3), MaterialIndex(1)));

        let actual_bounding_box = system_under_test.bounding_box();

        assert_eq!(actual_bounding_box.min(), Point::new(-9.0, -8.0, -7.0));
        assert_eq!(actual_bounding_box.max(), Point::new(0.2, 0.4, 0.6));
    }

    #[test]
    fn test_triangle_serialization() {
        let a = Vertex::new(Point::new(1.0, 2.0, 3.0), Vector::new(10.0, 11.0, 12.0));
        let b = Vertex::new(Point::new(4.0, 5.0, 6.0), Vector::new(13.0, 14.0, 15.0));
        let c = Vertex::new(Point::new(7.0, 8.0, 9.0), Vector::new(16.0, 17.0, 18.0));
        let expected_linkage = Linkage::new(ObjectUid(19), MaterialIndex(20));
        let system_under_test = Triangle::new(a, b, c, expected_linkage);

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
            f32::from_bits(expected_linkage.uid().0),
            c.normal().x as f32,
            c.normal().y as f32,
            c.normal().z as f32,
            f32::from_bits(expected_linkage.material_index().0 as u32),
        ];

        assert_eq!(actual_state.backend(), cast_slice::<f32, u8>(&expected));
    }
}
