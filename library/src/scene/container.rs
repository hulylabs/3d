use crate::geometry::alias::{Point, Vector};
use crate::geometry::axis::Axis;
use crate::geometry::fundamental_constants::VERTICES_IN_TRIANGLE;
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
use crate::bvh::builder::build_serialized_bvh;

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

    pub fn add_sphere(&mut self, center: Point, radius: f32, material: MaterialIndex) -> SphereIndex {
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

        self.triangles_count += obj.indices.len() / VERTICES_IN_TRIANGLE;

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

    fn transform<T: TransformableCoordinate>(victim: [f32; Axis::COUNT], transformation: &Affine) -> [f32; Axis::COUNT] {
        let entity = T::new(victim[Axis::X as usize], victim[Axis::Y as usize], victim[Axis::Z as usize]);
        let transformed_entity = entity.transform(transformation);
        transformed_entity.to_array()
    }
}

// TODO: unit tests