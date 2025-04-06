use crate::bvh::builder::build_serialized_bvh;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::Transformation;
use crate::objects::common_properties::{GlobalObjectIndex, Linkage};
use crate::objects::material::Material;
use crate::objects::material_index::MaterialIndex;
use crate::objects::parallelogram::{Parallelogram, ParallelogramIndex};
use crate::objects::sphere::{Sphere, SphereIndex};
use crate::objects::triangle::{MeshIndex, Triangle, TriangleIndex};
use crate::objects::triangle_mesh::TriangleMesh;
use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::serialization::serializable_for_gpu::SerializableForGpu;

#[derive(Default)]
pub struct Container {
    spheres: Vec<Sphere>,
    triangles: Vec<Triangle>,
    parallelograms: Vec<Parallelogram>,
    meshes: Vec<TriangleMesh>,
    materials: Vec<Material>,
    data_version: u64, // TODO: make per object kind granularity
}

impl Container {
    pub fn new() -> Self {
        Self {
            data_version: 0,
            ..Self::default()
        }
    }

    #[must_use]
    pub(crate) fn get_total_object_count(&self) -> usize {
        self.spheres.len() + self.parallelograms.len() + self.meshes.len()
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

    pub fn add_quadrilateral(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ParallelogramIndex {
        Container::add_object(&mut self.parallelograms, &mut self.data_version, |index| {
            Parallelogram::new(origin, local_x, local_y, Linkage::new(GlobalObjectIndex(0), index, material))
        })
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> MeshIndex {
        let index = Container::add_object(&mut self.meshes, &mut self.data_version, |index| {
            let links = Linkage::new(GlobalObjectIndex(0), index, material);
            let base_triangle_index = TriangleIndex(self.triangles.len());
            source.instantiate(slot, transformation, links, base_triangle_index)
        });

        let added = &self.meshes[index.0];
        added.put_triangles_into(&mut self.triangles);

        index
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
    pub(crate) fn evaluate_serialized_parallelograms(&self) -> Vec<f32> {
        Container::serialize(&self.parallelograms)
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
}

// TODO: more unit tests

#[cfg(test)]
mod tests {}
