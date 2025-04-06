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

pub(crate) struct GpuReadyTriangles {
    meshes: Vec<f32>,
    triangles: Vec<f32>,
    bvh: Vec<f32>,
}

impl GpuReadyTriangles {
    #[must_use]
    pub(crate) fn meshes(&self) -> &Vec<f32> {
        &self.meshes
    }
    #[must_use]
    pub(crate) fn geometry(&self) -> &Vec<f32> {
        &self.triangles
    }
    #[must_use]
    pub(crate) fn bvh(&self) -> &Vec<f32> {
        &self.bvh
    }
    #[must_use]
    pub(crate) fn empty(&self) -> bool {
        self.meshes.is_empty()
    }

    pub fn new(meshes: Vec<f32>, triangles: Vec<f32>, bvh: Vec<f32>) -> Self {
        Self { meshes, triangles, bvh }
    }
}

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
    #[must_use]
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
    pub(crate) fn spheres_count(&self) -> usize {
        self.spheres.len()
    }

    #[must_use]
    pub(crate) fn parallelograms_count(&self) -> usize {
        self.parallelograms.len()
    }

    #[must_use]
    pub(crate) fn materials_count(&self) -> usize {
        self.materials.len()
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

    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ParallelogramIndex {
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
    pub(crate) fn evaluate_serialized_triangles(&mut self) -> GpuReadyTriangles {
        let meshes = Container::serialize(&self.meshes);
        let bvh = build_serialized_bvh(&mut self.triangles);
        let triangles = Container::serialize(&self.triangles);
        GpuReadyTriangles::new(meshes, triangles, bvh)
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
mod tests {
    use crate::geometry::alias::Point;
    use crate::objects::common_properties::{GlobalObjectIndex, Linkage};
    use crate::objects::material::Material;
    use crate::objects::sphere::{Sphere, SphereIndex};
    use crate::scene::container::Container;
    use crate::serialization::serializable_for_gpu::SerializableForGpu;

    #[test]
    fn test_container_initialization() {
        let system_under_test = Container::new();
        assert_eq!(system_under_test.get_total_object_count(), 0);
    }

    #[test]
    fn test_add_sphere() {

        let mut system_under_test = Container::new();

        let dummy_material = system_under_test.add_material(&Material::default());
        let sphere_material = system_under_test.add_material(&Material::default().with_albedo(1.0, 0.0, 0.0));
        assert_ne!(dummy_material, sphere_material);

        let expected_sphere_center = Point::new(1.0, 2.0, 3.0);
        let expected_sphere_radius = 1.5;

        const SPHERES_TO_ADD: usize = 3;
        let mut expected_serialized_spheres = vec![0.0; Sphere::SERIALIZED_SIZE_FLOATS * SPHERES_TO_ADD];
        for i in 0..SPHERES_TO_ADD {
            let linkage = Linkage::new(GlobalObjectIndex(0), SphereIndex(i), sphere_material);
            let expected_sphere = Sphere::new(expected_sphere_center, expected_sphere_radius, linkage);
            expected_sphere.serialize_into(&mut expected_serialized_spheres[i*Sphere::SERIALIZED_SIZE_FLOATS..]);
        }

        for _ in 0..SPHERES_TO_ADD {
            let data_version_before_addition = system_under_test.data_version();
            system_under_test.add_sphere(expected_sphere_center, expected_sphere_radius, sphere_material);
            let data_version_after_addition = system_under_test.data_version();
            assert_ne!(data_version_before_addition, data_version_after_addition);
        }
        let serialized = system_under_test.evaluate_serialized_spheres();

        assert_eq!(system_under_test.get_total_object_count(), SPHERES_TO_ADD);
        assert_eq!(serialized, expected_serialized_spheres);
    }
}
