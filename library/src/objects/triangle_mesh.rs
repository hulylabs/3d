use crate::geometry::fundamental_constants::{COMPONENTS_IN_NORMAL, COMPONENTS_IN_POSITION, VERTICES_IN_TRIANGLE};
use crate::geometry::vertex::Vertex;
use crate::objects::common_properties::Linkage;
use crate::objects::triangle::{MeshIndex, Triangle, TriangleIndex, TriangleVertex};
use crate::serialization::helpers::{floats_count, GpuFloatBufferFiller};
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
    links: Linkage<MeshIndex>,
    triangles_base_index: TriangleIndex,
}

impl TriangleMesh {
    #[must_use]
    pub(crate) fn new(vertices: &[Vertex], indices: &[u32], mesh_links: Linkage<MeshIndex>, triangles_base_index: TriangleIndex) -> Self {
        assert_eq!(indices.len() % VERTICES_IN_TRIANGLE, 0, "illegal indices count of {}", indices.len());

        let mut triangles: Vec<Triangle> = Vec::new();
        for triangle in indices.chunks(VERTICES_IN_TRIANGLE) {
            let a = vertices[triangle[TriangleVertex::A as usize] as usize];
            let b = vertices[triangle[TriangleVertex::B as usize] as usize];
            let c = vertices[triangle[TriangleVertex::C as usize] as usize];
            let triangle_index = triangles_base_index + triangles.len();
            let mesh_index = mesh_links.in_kind_index();
            triangles.push(Triangle::new(a, b, c, triangle_index, mesh_index));
        }

        TriangleMesh {
            triangles,
            links: mesh_links,
            triangles_base_index,
        }
    }

    const SERIALIZED_QUARTET_COUNT: usize = 1;

    pub(crate) fn put_triangles_into(&self, target: &mut Vec<Triangle>) {
        target.extend(&self.triangles);
    }
}

impl SerializableForGpu for TriangleMesh {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(TriangleMesh::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= TriangleMesh::SERIALIZED_SIZE_FLOATS, "buffer size is too small");
        let mut index = 0;
        container.write_and_move_next(self.triangles.len() as f64, &mut index);
        container.write_and_move_next(self.triangles_base_index.as_f64(), &mut index);
        container.write_and_move_next(self.links.global_index().as_f64(), &mut index);
        container.write_and_move_next(self.links.material_index().as_f64(), &mut index);
        assert_eq!(index, TriangleMesh::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::common_properties::{GlobalObjectIndex, Linkage};
    use crate::objects::material_index::MaterialIndex;
    use cgmath::{EuclideanSpace, Zero};
    use crate::geometry::alias::{Point, Vector};

    const DUMMY_LINKS: Linkage<MeshIndex> = Linkage::new(GlobalObjectIndex(0), MeshIndex(0), MaterialIndex(0));

    #[test]
    fn test_triangle_mesh_new() {
        let triangle_indices = vec![0, 1, 2];
        let expected_links = Linkage::new(GlobalObjectIndex(1), MeshIndex(2), MaterialIndex(3));
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
            expected_triangles_base_index,
            expected_links.in_kind_index(),
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
        let container_initial_filler = -7.0;
        let expected_links = Linkage::new(GlobalObjectIndex(1), MeshIndex(2), MaterialIndex(3));
        let system_under_test = TriangleMesh::new(&vertices, &indices, expected_links, triangles_start);

        let mut container = vec![container_initial_filler; TriangleMesh::SERIALIZED_SIZE_FLOATS + 1];
        system_under_test.serialize_into(&mut container);

        let expected = vec![
            (indices.len() / VERTICES_IN_TRIANGLE) as f32,
            triangles_start.as_f64() as f32,
            expected_links.global_index().as_f64() as f32,
            expected_links.material_index().as_f64() as f32,
            container_initial_filler,
        ];

        assert_eq!(container, expected);
    }
}
