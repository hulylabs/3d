use crate::geometry::alias::{Point, Vector};
use crate::geometry::axis::Axis;
use crate::geometry::transform::{TransformableCoordinate, Transformation};
use crate::geometry::vertex::Vertex;
use crate::objects::common_properties::Linkage;
use crate::objects::triangle_mesh::{TriangleMesh, VertexData};
use obj::{Obj, ObjError};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use strum::EnumCount;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MeshLoadError {
    #[error("io problem while loading mesh: {what:?}")]
    IoError { what: String },
    #[error("format problem while loading mesh: {what:?}")]
    FormatError { what: String },
    #[error("invalid mesh content: {what:?}")]
    ContentError {what: String}
}

struct RawMesh {
    vertices: Vec<VertexData>,
    indices: Vec<u32>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WarehouseSlot(pub(crate) usize);

pub struct MeshWarehouse {
    prototypes: Vec<RawMesh>,
}

impl MeshWarehouse {
    #[must_use]
    pub fn new() -> Self {
        Self { prototypes: Vec::new() }
    }

    pub fn load(&mut self, source_file: &Path) -> Result<WarehouseSlot, MeshLoadError> {
        let file = File::open(source_file).map_err(|e| MeshLoadError::IoError { what: e.to_string() })?;
        let reader = BufReader::new(file);
        let obj: Obj<obj::Vertex, u32> = obj::load_obj::<obj::Vertex, BufReader<File>, u32>(reader).map_err(|e| MeshWarehouse::translate_error(e))?;

        if obj.indices.is_empty() || obj.vertices.is_empty() {
            return Err(MeshLoadError::ContentError { what: "empty mesh".to_string() });
        }

        let vertices: Vec<VertexData> = {
            let vertices_bytes = bytemuck::cast_slice(&obj.vertices);
            vertices_bytes.to_vec()
        };
        self.prototypes.push(RawMesh { vertices, indices: obj.indices });

        Ok(WarehouseSlot(self.prototypes.len() - 1))
    }

    #[must_use]
    pub(super) fn instantiate(&self, prototype: WarehouseSlot, transformation: &Transformation, links: Linkage,) -> TriangleMesh {
        let prototype_mesh = &self.prototypes[prototype.0];
        let transformed_vertices: Vec<Vertex> = prototype_mesh
            .vertices
            .iter()
            .map(|v| Vertex::new( MeshWarehouse::transform::<Point>(v.position, transformation), MeshWarehouse::transform::<Vector>(v.normal, transformation)))
            .collect();

        TriangleMesh::new(&transformed_vertices, &prototype_mesh.indices, links,)
    }

    #[must_use]
    fn transform<T: TransformableCoordinate>(victim: [f32; Axis::COUNT], transformation: &Transformation) -> T {
        let entity = T::new(victim[Axis::X as usize] as f64, victim[Axis::Y as usize] as f64, victim[Axis::Z as usize] as f64);
        entity.transform(transformation)
    }

    #[must_use]
    fn translate_error(from: ObjError) -> MeshLoadError {
        match from {
            ObjError::Io(_) => MeshLoadError::IoError { what: from.to_string() },
            ObjError::ParseInt(_) => MeshLoadError::FormatError { what: from.to_string() },
            ObjError::ParseFloat(_) => MeshLoadError::FormatError { what: from.to_string() },
            ObjError::Load(_) => MeshLoadError::FormatError { what: from.to_string() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::transform::Affine;
    use crate::objects::common_properties::ObjectUid;
    use crate::objects::material_index::MaterialIndex;
    use crate::objects::triangle::Triangle;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const _: () = assert!(size_of::<VertexData>() == size_of::<obj::Vertex>());

    const TEST_LINKS: Linkage = Linkage::new(ObjectUid(0), MaterialIndex(1));

    const SINGLE_TRIANGLE_OBJ_FILE: &str = r#"
        v  0.0  1.0  0.0
        v -1.0 -1.0  0.0
        v  1.0 -1.0  0.0

        vn  0.0  0.0  1.0

        f 1//1 2//1 3//1
        "#;
    
    #[test]
    fn test_add_mesh() {
        let mut temp_file = NamedTempFile::new_in("./").expect("failed to create temp file");
        temp_file.write_all(SINGLE_TRIANGLE_OBJ_FILE.as_bytes()).expect("failed to write dummy data into the temp file");

        let mut system_under_test = MeshWarehouse::new();
        let first_mesh_index = system_under_test.load(temp_file.path()).unwrap();
        let second_mesh_index = system_under_test.load(temp_file.path()).unwrap();
        assert_ne!(first_mesh_index, second_mesh_index);
        
        let transformation = Transformation::new(Affine::from_translation(Vector::new(1.0, 2.0, 3.0)));
        let instance = system_under_test.instantiate(second_mesh_index, &transformation, TEST_LINKS,);

        let mut triangles: Vec<Triangle> = vec![];
        instance.put_triangles_into(&mut triangles);

        assert_eq!(triangles.len(), 1);
    }
}
