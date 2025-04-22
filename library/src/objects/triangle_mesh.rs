use crate::geometry::fundamental_constants::{COMPONENTS_IN_NORMAL, COMPONENTS_IN_POSITION, VERTICES_IN_TRIANGLE};
use crate::geometry::vertex::Vertex;
use crate::objects::common_properties::Linkage;
use crate::objects::triangle::{Triangle, TriangleVertex};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct VertexData {
    pub(crate) position: [f32; COMPONENTS_IN_POSITION],
    pub(crate) normal: [f32; COMPONENTS_IN_NORMAL],
}

pub(crate) struct TriangleMesh {
    triangles: Vec<Triangle>,
}

impl TriangleMesh {
    #[must_use]
    pub(crate) fn new(vertices: &[Vertex], indices: &[u32], mesh_links: Linkage,) -> Self {
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
        }
    }

    pub(crate) fn put_triangles_into(&self, target: &mut Vec<Triangle>) {
        target.extend(&self.triangles);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::objects::common_properties::{Linkage, ObjectUid};
    use crate::objects::material_index::MaterialIndex;
    use cgmath::{EuclideanSpace, Zero};

    const DUMMY_LINKS: Linkage = Linkage::new(ObjectUid(0), MaterialIndex(0));

    #[test]
    fn test_triangle_mesh_new() {
        let triangle_indices = vec![0, 1, 2];
        let expected_links = Linkage::new(ObjectUid(1), MaterialIndex(3));
        let expected_vertices = [
            Vertex::new(Point::new(1.0, 2.0, 3.0), Vector::new(4.0, 5.0, 6.0)),
            Vertex::new(Point::new(7.0, 8.0, 9.0), Vector::new(10.0, 11.0, 12.0)),
            Vertex::new(Point::new(13.0, 14.0, 15.0), Vector::new(16.0, 17.0, 18.0)),
        ];

        let system_under_test = TriangleMesh::new(&expected_vertices, &triangle_indices, expected_links,);

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
    }

    #[test]
    #[should_panic(expected = "illegal indices count of 2")]
    fn test_triangle_mesh_new_illegal_indices_count() {
        let vertices = [
            Vertex::new(Point::origin(), Vector::zero()),
        ];
        let indices = vec![0, 0];
        let _system_under_test = TriangleMesh::new(&vertices, &indices, DUMMY_LINKS,);
    }
}
