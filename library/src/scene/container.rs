use crate::bvh::builder::build_serialized_bvh;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::axis::Axis;
use crate::geometry::transform::{Affine, TransformableCoordinate};
use crate::objects::common_properties::{GlobalObjectIndex, Linkage};
use crate::objects::material::Material;
use crate::objects::material_index::MaterialIndex;
use crate::objects::quadrilateral::{Quadrilateral, QuadrilateralIndex};
use crate::objects::sphere::{Sphere, SphereIndex};
use crate::objects::triangle::{MeshIndex, Triangle, TriangleIndex};
use crate::objects::triangle_mesh::{TriangleMesh, VertexData};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
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
}

#[derive(Default)]
pub struct Container {
    spheres: Vec<Sphere>,
    triangles: Vec<Triangle>,
    quadrilaterals: Vec<Quadrilateral>,
    meshes: Vec<TriangleMesh>,
    materials: Vec<Material>,
    data_version: u64, // TODO: make per object kind granularity
    triangles_count: usize,
}

impl Container {
    pub fn new() -> Self {
        Self {
            data_version: 0,
            triangles_count: 0,
            ..Self::default()
        }
    }

    #[must_use]
    pub(crate) fn get_total_object_count(&self) -> usize {
        self.spheres.len() + self.quadrilaterals.len() + self.meshes.len()
    }

    #[must_use]
    pub fn add_material(&mut self, target: &Material) -> MaterialIndex {
        Container::add_object(&mut self.materials, &mut self.data_version, |_| *target)
    }

    pub fn add_sphere(&mut self, center: Point, radius: f64, material: MaterialIndex) -> SphereIndex {
        assert!(radius > 0.0, "radius must be positive");
        Container::add_object(&mut self.spheres, &mut self.data_version, |index| {
            Sphere::new(center, radius, Linkage::new(GlobalObjectIndex(0), index, material)) // TODO: refactor: get rid of global indices
        })
    }

    pub fn add_quadrilateral(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> QuadrilateralIndex {
        Container::add_object(&mut self.quadrilaterals, &mut self.data_version, |index| {
            Quadrilateral::new(origin, local_x, local_y, Linkage::new(GlobalObjectIndex(0), index, material))
        })
    }

    pub fn add_mesh(&mut self, source_file: &Path, transformation: &Affine, material: MaterialIndex) -> Result<MeshIndex, MeshLoadError> {
        let file = File::open(source_file).map_err(|e| MeshLoadError::IoError { what: e.to_string() })?;
        let reader = BufReader::new(file);
        let obj: Obj<obj::Vertex, u32> = obj::load_obj::<obj::Vertex, BufReader::<File>, u32>(reader).map_err(|e| Container::translate_error(e))?;

        let index = Container::add_object(&mut self.meshes, &mut self.data_version, |index| {
            let vertices: Vec<VertexData> = obj.vertices.iter().map(|v| VertexData {
                position: Container::transform::<Point>(v.position, transformation),
                normal: Container::transform::<Vector>(v.normal, transformation),
            }).collect();

            TriangleMesh::new(&vertices, &obj.indices, Linkage::new(GlobalObjectIndex(0), index, material,), TriangleIndex(self.triangles_count))
        });

        let added = &self.meshes[index.0];
        self.triangles.extend(added.triangles());
        self.triangles_count += added.triangles().len();

        Ok(index)
    }

    #[must_use]
    fn add_object<Object, ObjectIndex, Constructor>(container: &mut Vec<Object>, data_version: &mut u64, create_object: Constructor) -> ObjectIndex
    where
        ObjectIndex: From<usize> + Copy,
        Constructor: FnOnce(ObjectIndex) -> Object,
    {
        let index = ObjectIndex::from(container.len());
        let object = create_object(index);
        container.push(object);
        *data_version += 1;

        index
    }

    #[must_use]
    pub fn data_version(&self) -> u64 {
        self.data_version
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_bvh(&mut self) -> Vec<f32> {
        build_serialized_bvh(&mut self.triangles) // TODO: is it ok, to reorder triangles here? -> seems ok: no ids usage in the shader
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_triangles(&self) -> Vec<f32> {
        Container::serialize(&self.triangles)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_meshes(&self) -> Vec<f32> {
        Container::serialize(&self.meshes)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_materials(&self) -> Vec<f32> {
        Container::serialize(&self.materials)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_spheres(&self) -> Vec<f32> {
        Container::serialize(&self.spheres)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_quadrilaterals(&self) -> Vec<f32> {
        Container::serialize(&self.quadrilaterals)
    }

    #[must_use]
    fn serialize<T: SerializableForGpu>(items: &Vec<T>) -> Vec<f32> {
        // TODO: we can reuse the buffer in case object count is the same
        let mut buffer: Vec<f32> = vec![0.0; T::SERIALIZED_SIZE_FLOATS * items.len()];
        let mut index = 0;

        for item in items {
            item.serialize_into(&mut buffer[index..(index + T::SERIALIZED_SIZE_FLOATS)]);
            index += T::SERIALIZED_SIZE_FLOATS;
        }

        buffer
    }

    fn translate_error(from: ObjError) -> MeshLoadError {
        match from {
            ObjError::Io(_) => MeshLoadError::IoError { what: from.to_string() },
            ObjError::ParseInt(_) => MeshLoadError::FormatError { what: from.to_string() },
            ObjError::ParseFloat(_) => MeshLoadError::FormatError { what: from.to_string() },
            ObjError::Load(_) => MeshLoadError::FormatError { what: from.to_string() },
        }
    }

    fn transform<T: TransformableCoordinate>(victim: [f32; Axis::COUNT], transformation: &Affine) -> [f64; Axis::COUNT] {
        let entity = T::new(victim[Axis::X as usize] as f64, victim[Axis::Y as usize] as f64, victim[Axis::Z as usize] as f64);
        let transformed_entity = entity.transform(transformation);
        transformed_entity.to_array()
    }
}

// TODO: more unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use cgmath::SquareMatrix;
    use tempfile::NamedTempFile;

    const DUMMY_OBJ_FILE: &str = r#"
        v  0.0  1.0  0.0
        v -1.0 -1.0  0.0
        v  1.0 -1.0  0.0

        vn  0.0  0.0  1.0

        f 1//1 2//1 3//1
        "#;

    #[test]
    fn test_add_mesh() {
        let mut temp_file = NamedTempFile::new_in("./").expect("failed to create temp file");
        temp_file.write_all(DUMMY_OBJ_FILE.as_bytes()).expect("failed to write dummy data into the temp file");

        let mut system_under_test = Container::new();
        let dummy_material = system_under_test.add_material(&Material::default());
        let first_mesh_index = system_under_test.add_mesh(temp_file.path(), &Affine::from_translation(Vector::new(1.0, 2.0, 3.0)), dummy_material);

        assert_eq!(system_under_test.meshes.len(), 1);
        assert_eq!(system_under_test.triangles.len(), 1);
        assert_eq!(system_under_test.triangles_count, system_under_test.triangles.len());

        let second_mesh_index = system_under_test.add_mesh(temp_file.path(), &Affine::from_translation(Vector::new(1.0, 2.0, 3.0)), dummy_material);

        assert_eq!(system_under_test.meshes.len(), 2);
        assert_eq!(system_under_test.triangles.len(), 2);
        assert_eq!(system_under_test.triangles_count, system_under_test.triangles.len());

        assert_ne!(first_mesh_index.unwrap(), second_mesh_index.unwrap());
    }

    const CUBE_OBJ_FILE: &str = r#"
            v 0.270893 0.270893 -0.270893
            v 0.270893 -0.270893 -0.270893
            v 0.270893 0.270893 0.270893
            v 0.270893 -0.270893 0.270893
            v -0.270893 0.270893 -0.270893
            v -0.270893 -0.270893 -0.270893
            v -0.270893 0.270893 0.270893
            v -0.270893 -0.270893 0.270893
            vn -0.0000 1.0000 -0.0000
            vn -0.0000 -0.0000 1.0000
            vn -1.0000 -0.0000 -0.0000
            vn -0.0000 -1.0000 -0.0000
            vn 1.0000 -0.0000 -0.0000
            vn -0.0000 -0.0000 -1.0000
            vt 0.625000 0.500000
            vt 0.375000 0.500000
            vt 0.625000 0.750000
            vt 0.375000 0.750000
            vt 0.875000 0.500000
            vt 0.625000 0.250000
            vt 0.125000 0.500000
            vt 0.375000 0.250000
            vt 0.875000 0.750000
            vt 0.625000 1.000000
            vt 0.625000 0.000000
            vt 0.375000 0.000000
            vt 0.375000 1.000000
            vt 0.125000 0.750000
            s 0
            f 5/5/1 3/3/1 1/1/1
            f 3/3/2 8/13/2 4/4/2
            f 7/11/3 6/8/3 8/12/3
            f 2/2/4 8/14/4 6/7/4
            f 1/1/5 4/4/5 2/2/5
            f 5/6/6 2/2/6 6/8/6
            f 5/5/1 7/9/1 3/3/1
            f 3/3/2 7/10/2 8/13/2
            f 7/11/3 5/6/3 6/8/3
            f 2/2/4 4/4/4 8/14/4
            f 1/1/5 3/3/5 4/4/5
            f 5/6/6 1/1/6 2/2/6
        "#;

    const SERIALIZED_CUBE_TRIANGLES: [f32; 288] = [-0.27089300751686096_f32, 0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, -1.0, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, 2.0, -1.0, 0.0, 0.0, 0.0, -0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, -1.0, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, 8.0, -1.0, 0.0, 0.0, 0.0, 0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, 0.0, -1.0, 0.0, -1.0, 0.0, -1.0, 0.0, 3.0, 0.0, -1.0, 0.0, 0.0, -0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, -1.0, 5.0, 0.0, 0.0, -1.0, 0.0, -0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, 0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, 0.0, -1.0, 0.0, -1.0, 0.0, -1.0, 0.0, 9.0, 0.0, -1.0, 0.0, 0.0, -0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, 0.0, 0.0, -1.0, -1.0, 0.0, 0.0, -1.0, 11.0, 0.0, 0.0, -1.0, 0.0, 0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, -0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 1.0, 7.0, 0.0, 0.0, 1.0, 0.0, -0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, -0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, 0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 6.0, 0.0, 1.0, 0.0, 0.0, 0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, -0.27089300751686096, -1.0, 1.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 4.0, 1.0, 0.0, 0.0, 0.0, 0.27089300751686096, 0.27089300751686096, -0.27089300751686096, -1.0, 0.27089300751686096, 0.27089300751686096, 0.27089300751686096, -1.0, 0.27089300751686096, -0.27089300751686096, 0.27089300751686096, -1.0, 1.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 10.0, 1.0, 0.0, 0.0, 0.0,];

    #[test]
    fn test_cube_mesh() {
        let mut temp_file = NamedTempFile::new_in("./").expect("failed to create temp file");
        temp_file.write_all(CUBE_OBJ_FILE.as_bytes()).expect("failed to write dummy data into the temp file");

        let mut system_under_test = Container::new();
        let dummy_material = system_under_test.add_material(&Material::default());

        let mesh_index = system_under_test.add_mesh(temp_file.path(), &Affine::identity(), dummy_material);
        assert!(!mesh_index.is_err());

        let actual_triangles = system_under_test.evaluate_serialized_triangles();
        assert_eq!(actual_triangles, SERIALIZED_CUBE_TRIANGLES);
    }
}
