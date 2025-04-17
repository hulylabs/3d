use crate::geometry::fundamental_constants::{COMPONENTS_IN_NORMAL, COMPONENTS_IN_POSITION, VERTICES_IN_TRIANGLE};
use crate::geometry::vertex::Vertex;
use crate::objects::common_properties::Linkage;
use crate::objects::triangle::{Triangle, TriangleIndex, TriangleVertex};
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct VertexData {
    pub(crate) position: [f32; COMPONENTS_IN_POSITION],
    pub(crate) normal: [f32; COMPONENTS_IN_NORMAL],
}

pub(crate) struct TriangleMesh {
    triangles: Vec<Triangle>,
    links: Linkage,
    triangles_base_index: TriangleIndex,
}

impl TriangleMesh {
    #[must_use]
    pub(crate) fn new(vertices: &[Vertex], indices: &[u32], mesh_links: Linkage, triangles_base_index: TriangleIndex) -> Self {
        assert_eq!(indices.len() % VERTICES_IN_TRIANGLE, 0, "illegal indices count of {}", indices.len());

        let mut triangles: Vec<Triangle> = Vec::new();
        for triangle in indices.chunks(VERTICES_IN_TRIANGLE) {
            let a = vertices[triangle[TriangleVertex::A as usize] as usize];
            let b = vertices[triangle[TriangleVertex::B as usize] as usize];
            let c = vertices[triangle[TriangleVertex::C as usize] as usize];
            triangles.push(Triangle::new(a, b, c, mesh_links));
        }

        TriangleMesh {
            triangles,
            links: mesh_links,
            triangles_base_index,
        }
    }

    pub(crate) fn put_triangles_into(&self, target: &mut Vec<Triangle>) {
        target.extend(&self.triangles);
    }
}

impl SerializableForGpu for TriangleMesh {
    const SERIALIZED_QUARTET_COUNT: usize = 1;

    fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        debug_assert!(container.has_free_slot(), "buffer overflow");

        container.write_quartet_f64(
            self.triangles.len() as f64,
            self.triangles_base_index.as_f64(),
            self.links.uid().0 as f64,
            self.links.material_index().as_f64(),
        );

        debug_assert!(container.object_fully_written());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::objects::common_properties::{ObjectUid, Linkage};
    use crate::objects::material_index::MaterialIndex;
    use bytemuck::cast_slice;
    use cgmath::{EuclideanSpace, Zero};

    const DUMMY_LINKS: Linkage = Linkage::new(ObjectUid(0), MaterialIndex(0));

    #[test]
    fn test_triangle_mesh_new() {
        let triangle_indices = vec![0, 1, 2];
        let expected_links = Linkage::new(ObjectUid(1), MaterialIndex(3));
        let expected_triangles_base_index = TriangleIndex(4);
        let expected_vertices = [
            Vertex::new(Point::new(1.0, 2.0, 3.0), Vector::new(4.0, 5.0, 6.0)),
            Vertex::new(Point::new(7.0, 8.0, 9.0), Vector::new(10.0, 11.0, 12.0)),
            Vertex::new(Point::new(13.0, 14.0, 15.0), Vector::new(16.0, 17.0, 18.0)),
        ];

        let system_under_test = TriangleMesh::new(&expected_vertices, &triangle_indices, expected_links, expected_triangles_base_index);

        let expected_triangles = vec![Triangle::new(
            expected_vertices[0],
            expected_vertices[1],
            expected_vertices[2],
            expected_links,
        )];

        assert_eq!(system_under_test.triangles.len(), expected_triangles.len());
        for i in 0..expected_triangles.len() {
            assert_eq!(system_under_test.triangles[i], expected_triangles[i]);
        }
        assert_eq!(system_under_test.links, expected_links);
        assert_eq!(system_under_test.triangles_base_index, expected_triangles_base_index);
    }

    #[test]
    #[should_panic(expected = "illegal indices count of 2")]
    fn test_triangle_mesh_new_illegal_indices_count() {
        let vertices = [
            Vertex::new(Point::origin(), Vector::zero()),
        ];
        let indices = vec![0, 0];
        let _system_under_test = TriangleMesh::new(&vertices, &indices, DUMMY_LINKS, TriangleIndex(0));
    }

    #[test]
    fn test_serialize_into() {
        let vertices = [
            Vertex::new(Point::origin(), Vector::zero()),
        ];
        let indices = vec![0, 0, 0];
        let triangles_start = TriangleIndex(3);
        let expected_links = Linkage::new(ObjectUid(1), MaterialIndex(3));
        let system_under_test = TriangleMesh::new(&vertices, &indices, expected_links, triangles_start);

        let mut container = GpuReadySerializationBuffer::new(1, TriangleMesh::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);

        let expected = vec![
            (indices.len() / VERTICES_IN_TRIANGLE) as f32,
            triangles_start.as_f64() as f32,
            expected_links.uid().0 as f64 as f32,
            expected_links.material_index().as_f64() as f32,
        ];

        assert_eq!(container.backend(), cast_slice(&expected));
    }
}
