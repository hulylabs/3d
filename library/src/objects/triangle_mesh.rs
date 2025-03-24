use bytemuck::{Pod, Zeroable};
use crate::geometry::aabb::Axis;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::fundamental_constants::{COMPONENTS_IN_NORMAL, COMPONENTS_IN_POSITION, VERTICES_IN_TRIANGLE};
use crate::geometry::transform::Transformation;
use crate::geometry::vertex::Vertex;
use crate::objects::triangle::{MeshIndex, Triangle, TriangleIndex, TriangleVertex};
use crate::objects::utils::common_properties::Linkage;
use crate::objects::utils::serialization_helpers::GpuFloatBufferFiller;
use crate::panic_if_failed;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct VertexData {
    pub(crate) position: [f32; COMPONENTS_IN_POSITION],
    pub(crate) normal: [f32; COMPONENTS_IN_NORMAL],
}
impl VertexData {
    fn as_vertex(&self) -> Vertex {
        Vertex::new(
            Point::new(self.position[Axis::X as usize], self.position[Axis::Y as usize], self.position[Axis::Z as usize]),
            Vector::new(self.normal[Axis::X as usize], self.normal[Axis::Y as usize], self.normal[Axis::Z as usize])
        )
    }
}

pub(crate) struct TriangleMesh {
    triangles: Vec<Triangle>,
    links: Linkage<MeshIndex>,
    triangles_base_index: TriangleIndex,
}

impl TriangleMesh {
    #[must_use]
    pub(crate) fn new(
        vertices: &[VertexData],
        indices: &Vec<u32>,
        links: Linkage<MeshIndex>,
        triangles_base_index: TriangleIndex,
    ) -> Self {
        panic_if_failed!(indices.len() % VERTICES_IN_TRIANGLE == 0, "illegal indices count of {}", indices.len());

        let mut triangles: Vec<Triangle> = Vec::new();
        for triangle in indices.chunks(VERTICES_IN_TRIANGLE) {
            let a = vertices[triangle[TriangleVertex::A as usize] as usize].as_vertex();
            let b = vertices[triangle[TriangleVertex::B as usize] as usize].as_vertex();
            let c = vertices[triangle[TriangleVertex::C as usize] as usize].as_vertex();
            triangles.push(Triangle::new(a, b, c, triangles_base_index + triangles.len(), links.in_kind_index()));
        }

        TriangleMesh {triangles, links, triangles_base_index}
    }

    pub(crate) fn transform(&mut self, transformation: &Transformation) {
        for triangle in &mut self.triangles {
            *triangle = triangle.transform(transformation);
        }
    }

    const SERIALIZED_QUARTET_COUNT: usize = 1;
    pub(crate) const SERIALIZED_SIZE: usize = TriangleMesh::SERIALIZED_QUARTET_COUNT * <[f32] as GpuFloatBufferFiller>::FLOAT_ALIGNMENT_SIZE;

    pub(crate) fn serialize_into(&self, container: &mut [f32]) {
        panic_if_failed!(container.len() >= TriangleMesh::SERIALIZED_SIZE, "buffer size is too small");
        let mut index = 0;
        container.write_and_move_next(self.triangles.len() as f32,          &mut index);
        container.write_and_move_next(self.triangles_base_index.as_f32(),   &mut index);
        container.write_and_move_next(self.links.global_index().as_f32(),   &mut index);
        container.write_and_move_next(self.links.material_index().as_f32(), &mut index);
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::transform::Affine;
    use crate::objects::utils::common_properties::{GlobalObjectIndex, MaterialIndex};
    use super::*;

    const THREE_VERTICES_DATA: [f32; 18] = [
         1.0,  2.0,  3.0,
         4.0,  5.0,  6.0,

         7.0,  8.0,  9.0,
        10.0, 11.0, 12.0,

        11.0, 12.0, 13.0,
        14.0, 15.0, 16.0,
    ];

    const DUMMY_LINKS: Linkage<MeshIndex> = Linkage::new(GlobalObjectIndex(0), MeshIndex(0), MaterialIndex(0));

    #[test]
    fn test_triangle_mesh_new() {
        let triangle_indices = vec![0, 1, 2,];
        let expected_links = Linkage::new(GlobalObjectIndex(1), MeshIndex(2), MaterialIndex(3));
        let triangles_base_index = TriangleIndex(4);
        let vertices: &[VertexData] = bytemuck::cast_slice(&THREE_VERTICES_DATA);

        let system_under_test = TriangleMesh::new(vertices, &triangle_indices, expected_links.clone(), triangles_base_index);

        let expected_triangles = vec![
            Triangle::new(
                Vertex::new(Point::new( 1.0,  2.0,  3.0), Vector::new( 4.0,  5.0,  6.0)),
                Vertex::new(Point::new( 7.0,  8.0,  9.0), Vector::new(10.0, 11.0, 12.0)),
                Vertex::new(Point::new(11.0, 12.0, 13.0), Vector::new(14.0, 15.0, 16.0)),
                TriangleIndex(4),
                expected_links.in_kind_index(),),
        ];

        assert_eq!(system_under_test.triangles.len(), expected_triangles.len());
        for i in 0..expected_triangles.len() {
            assert_eq!(system_under_test.triangles[i], expected_triangles[i]);
        }
        assert_eq!(system_under_test.links, expected_links);
        assert_eq!(system_under_test.triangles_base_index, triangles_base_index);
    }

    #[test]
    #[should_panic(expected = "illegal indices count of 2")]
    fn test_triangle_mesh_new_illegal_indices_count() {
        let vertices = [VertexData { position: [1.0, 1.0, 1.0], normal: [1.0, 1.0, 1.0]},];
        let indices = vec![0, 0];
        let system_under_test = TriangleMesh::new(&vertices, &indices, DUMMY_LINKS, TriangleIndex(0));
    }

    #[test]
    fn test_transform() {
        let vertices: &[VertexData] = bytemuck::cast_slice(&THREE_VERTICES_DATA);
        let indices = vec![0, 1, 2, 0, 1, 2];

        let mut system_under_test = TriangleMesh::new(&vertices, &indices, DUMMY_LINKS, TriangleIndex(4));
        let matrix = Affine::from_translation(Vector::new(-1.0, -2.0, -3.0));

        system_under_test.transform(&Transformation::new(matrix));

        let expected_triangles = vec![
            Triangle::new(
                Vertex::new(Point::new( 0.0,  0.0,  0.0), Vector::new( 4.0,  5.0,  6.0)),
                Vertex::new(Point::new( 6.0,  6.0,  6.0), Vector::new(10.0, 11.0, 12.0)),
                Vertex::new(Point::new(10.0, 10.0, 10.0), Vector::new(14.0, 15.0, 16.0)),
                TriangleIndex(4),
                DUMMY_LINKS.in_kind_index(),),
            Triangle::new(
                Vertex::new(Point::new( 0.0,  0.0,  0.0), Vector::new( 4.0,  5.0,  6.0)),
                Vertex::new(Point::new( 6.0,  6.0,  6.0), Vector::new(10.0, 11.0, 12.0)),
                Vertex::new(Point::new(10.0, 10.0, 10.0), Vector::new(14.0, 15.0, 16.0)),
                TriangleIndex(5),
                DUMMY_LINKS.in_kind_index(),)
        ];

        assert_eq!(system_under_test.triangles.len(), expected_triangles.len());
        for i in 0..expected_triangles.len() {
            assert_eq!(system_under_test.triangles[i], expected_triangles[i]);
        }
    }

    #[test]
    fn test_serialize_into() {
        let vertices = [VertexData { position: [1.0, 1.0, 1.0], normal: [1.0, 1.0, 1.0]},];
        let indices = vec![0, 0, 0];
        let triangles_start = TriangleIndex(3);
        let container_initial_filler = -7.0;
        let expected_links = Linkage::new(GlobalObjectIndex(1), MeshIndex(2), MaterialIndex(3));
        let system_under_test = TriangleMesh::new(&vertices, &indices, expected_links, triangles_start);

        let mut container = vec![container_initial_filler; TriangleMesh::SERIALIZED_SIZE + 1];
        system_under_test.serialize_into(&mut container);

        let expected = vec![
            (indices.len() / VERTICES_IN_TRIANGLE) as f32,
            triangles_start.as_f32(),
            expected_links.global_index().as_f32(),
            expected_links.material_index().as_f32(),
            container_initial_filler
        ];

        assert_eq!(container, expected);
    }
}