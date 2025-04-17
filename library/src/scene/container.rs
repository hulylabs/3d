use crate::bvh::builder::build_serialized_bvh;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::objects::common_properties::{Linkage, ObjectUid};
use crate::objects::material::Material;
use crate::objects::material_index::MaterialIndex;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::{SdfBox, SdfBoxIndex};
use crate::objects::sphere::Sphere;
use crate::objects::triangle::{Triangle, TriangleIndex};
use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::SerializableForGpu;

pub(crate) struct GpuReadyTriangles {
    triangles: GpuReadySerializationBuffer,
    bvh: GpuReadySerializationBuffer,
}

impl GpuReadyTriangles {
    #[must_use]
    pub(crate) fn geometry(&self) -> &GpuReadySerializationBuffer {
        &self.triangles
    }
    #[must_use]
    pub(crate) fn bvh(&self) -> &GpuReadySerializationBuffer {
        &self.bvh
    }
    #[must_use]
    pub(crate) fn empty(&self) -> bool {
        self.triangles.is_empty()
    }

    pub fn new(triangles: GpuReadySerializationBuffer, bvh: GpuReadySerializationBuffer) -> Self {
        Self { triangles, bvh }
    }
}

#[derive(Default)]
pub struct Container {
    spheres: Vec<Sphere>,
    triangles: Vec<Triangle>,
    parallelograms: Vec<Parallelogram>,
    sdf: Vec<SdfBox>,
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
    pub(crate) fn spheres_count(&self) -> usize {
        self.spheres.len()
    }

    #[must_use]
    pub(crate) fn parallelograms_count(&self) -> usize {
        self.parallelograms.len()
    }

    #[must_use]
    pub(crate) fn sdf_objects_count(&self) -> usize {
        self.sdf.len()
    }

    #[must_use]
    pub(crate) fn materials_count(&self) -> usize {
        self.materials.len()
    }

    #[must_use]
    pub fn add_material(&mut self, target: &Material) -> MaterialIndex {
        Container::add_object(&mut self.materials, &mut self.data_version, |_| *target)
    }

    pub fn add_sphere(&mut self, center: Point, radius: f64, material: MaterialIndex) -> ObjectUid {
        assert!(radius > 0.0, "radius must be positive");
        Container::add_object(&mut self.spheres, &mut self.data_version, |_index| {
            Sphere::new(center, radius, Linkage::new(ObjectUid(0), material)) // TODO: refactor: get rid of global indices
        })
    }

    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ObjectUid {
        Container::add_object(&mut self.parallelograms, &mut self.data_version, |_index| {
            Parallelogram::new(origin, local_x, local_y, Linkage::new(ObjectUid(0), material))
        })
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> ObjectUid {
        let links = Linkage::new(ObjectUid(0), material);
        let base_triangle_index = TriangleIndex(self.triangles.len());
        let instance = source.instantiate(slot, transformation, links, base_triangle_index);
        instance.put_triangles_into(&mut self.triangles);

        links.uid()
    }

    pub fn add_sdf_box(&mut self, location: &Affine, half_size: Vector, corners_radius: f64, material: MaterialIndex) -> SdfBoxIndex {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0);
        assert!(corners_radius >= 0.0);
        Container::add_object(&mut self.sdf, &mut self.data_version, |_index| {
            SdfBox::new(*location, half_size, corners_radius, material)
        })
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
        let bvh = build_serialized_bvh(&mut self.triangles);
        let triangles = Container::serialize(&self.triangles);
        GpuReadyTriangles::new(triangles, bvh)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_materials(&self) -> GpuReadySerializationBuffer {
        Container::serialize(&self.materials)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_spheres(&self) -> GpuReadySerializationBuffer {
        Container::serialize(&self.spheres)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_parallelograms(&self) -> GpuReadySerializationBuffer {
        Container::serialize(&self.parallelograms)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_sdf(&self) -> GpuReadySerializationBuffer {
        Container::serialize(&self.sdf)
    }

    #[must_use]
    fn serialize<T: SerializableForGpu>(items: &Vec<T>) -> GpuReadySerializationBuffer {
        // TODO: we can reuse the buffer in case object count is the same
        let mut buffer = GpuReadySerializationBuffer::new(items.len(), T::SERIALIZED_QUARTET_COUNT);

        for item in items {
            item.serialize_into(&mut buffer);
        }

        buffer
    }
}

// TODO: more unit tests

#[cfg(test)]
mod tests {
    use crate::geometry::alias::Point;
    use crate::objects::common_properties::{Linkage, ObjectUid};
    use crate::objects::material::Material;
    use crate::objects::sphere::Sphere;
    use crate::scene::container::Container;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::serializable_for_gpu::SerializableForGpu;

    #[test]
    fn test_add_sphere() {

        let mut system_under_test = Container::new();

        let dummy_material = system_under_test.add_material(&Material::default());
        let sphere_material = system_under_test.add_material(&Material::default().with_albedo(1.0, 0.0, 0.0));
        assert_ne!(dummy_material, sphere_material);

        let expected_sphere_center = Point::new(1.0, 2.0, 3.0);
        let expected_sphere_radius = 1.5;

        const SPHERES_TO_ADD: usize = 3;
        let mut expected_serialized_spheres = GpuReadySerializationBuffer::new(SPHERES_TO_ADD, Sphere::SERIALIZED_QUARTET_COUNT);
        for _ in 0..SPHERES_TO_ADD {
            let linkage = Linkage::new(ObjectUid(0), sphere_material);
            let expected_sphere = Sphere::new(expected_sphere_center, expected_sphere_radius, linkage);
            expected_sphere.serialize_into(&mut expected_serialized_spheres);
        }

        for _ in 0..SPHERES_TO_ADD {
            let data_version_before_addition = system_under_test.data_version();
            system_under_test.add_sphere(expected_sphere_center, expected_sphere_radius, sphere_material);
            let data_version_after_addition = system_under_test.data_version();
            assert_ne!(data_version_before_addition, data_version_after_addition);
        }
        let serialized = system_under_test.evaluate_serialized_spheres();

        assert_eq!(serialized.backend(), expected_serialized_spheres.backend());
    }
}
