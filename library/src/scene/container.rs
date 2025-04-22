use crate::bvh::builder::build_serialized_bvh;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::objects::common_properties::Linkage;
use crate::objects::material_index::MaterialIndex;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::SdfBox;
use crate::objects::sphere::Sphere;
use crate::objects::triangle::Triangle;
use crate::scene::gpu_ready_triangles::GpuReadyTriangles;
use crate::scene::materials_warehouse::MaterialsWarehouse;
use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::scene::monolithic::Monolithic;
use crate::scene::scene_object::SceneObject;
use crate::scene::statistics::Statistics;
use crate::scene::triangulated::Triangulated;
use crate::scene::version::Version;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;
use crate::utils::object_uid::ObjectUid;
use crate::utils::uid_generator::UidGenerator;
use std::collections::HashMap;
use strum::EnumCount;
use strum_macros::{AsRefStr, Display, EnumCount, EnumIter};

#[derive(EnumIter, EnumCount, Display, AsRefStr, Copy, Clone, PartialEq, Debug)]
pub(crate) enum DataKind {
    Sphere,
    Parallelogram,
    Sdf,
    TriangleMesh,
}

pub struct Container {
    per_object_kind_statistics: Vec<Statistics>,
    objects: HashMap<ObjectUid, Box<dyn SceneObject>>,
    triangles: Vec<Triangle>,
    
    materials: MaterialsWarehouse,
    
    uid_generator: UidGenerator,
}

impl Container {
    #[must_use]
    pub fn new() -> Self {
        let per_object_kind_statistics: Vec<Statistics> = vec![Statistics::default(); DataKind::COUNT];

        Self {
            per_object_kind_statistics,
            objects: HashMap::new(),
            triangles: Vec::new(),
            materials: MaterialsWarehouse::new(),
            uid_generator: UidGenerator::new(),
        }
    }

    #[must_use]
    pub fn materials(&mut self) -> &mut MaterialsWarehouse {
        &mut self.materials
    }

    pub fn set_material(&mut self, victim: ObjectUid, material: MaterialIndex) {
        match self.objects.get_mut(&victim) {
            Some(object) => {
                object.set_material(material, &mut self.triangles);
                self.per_object_kind_statistics[object.data_kind_uid()].register_object_mutation();
            },
            None => panic!("object {} not found", victim),
        }
    }

    #[must_use]
    pub fn material_of(&self, victim: ObjectUid) -> MaterialIndex {
        match self.objects.get(&victim) {
            Some(object) => {
                object.material()
            },
            None => panic!("object {} not found", victim),
        }
    }

    pub fn add_sphere(&mut self, center: Point, radius: f64, material: MaterialIndex) -> ObjectUid {
        assert!(radius > 0.0, "radius must be positive");
        Self::add_object(&mut self.objects, &mut self.uid_generator, &mut self.per_object_kind_statistics, |uid| {
            Box::new(Monolithic::new(
                DataKind::Sphere as usize,
                Box::new(Sphere::new(center, radius, Linkage::new(uid, material))),
            ))
        })
    }

    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ObjectUid {
        Self::add_object(&mut self.objects, &mut self.uid_generator, &mut self.per_object_kind_statistics, |uid| {
            Box::new(Monolithic::new(
                DataKind::Parallelogram as usize,
                Box::new(Parallelogram::new(origin, local_x, local_y, Linkage::new(uid, material))),
            ))
        })
    }

    pub fn add_sdf_box(&mut self, location: &Affine, half_size: Vector, corners_radius: f64, material: MaterialIndex) -> ObjectUid {
        assert!(half_size.x > 0.0 && half_size.y > 0.0 && half_size.z > 0.0);
        assert!(corners_radius >= 0.0);
        Self::add_object(&mut self.objects, &mut self.uid_generator, &mut self.per_object_kind_statistics, |uid| {
            Box::new(Monolithic::new(
                DataKind::Sdf as usize,
                Box::new(SdfBox::new(*location, half_size, corners_radius, Linkage::new(uid, material))),
            ))
        })
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> ObjectUid {
        let links = Linkage::new(self.uid_generator.next(), material);

        let instance = source.instantiate(slot, transformation, links,);
        instance.put_triangles_into(&mut self.triangles);

        let geometry_kind = DataKind::TriangleMesh as usize;
        self.objects.insert(links.uid(), Box::new(Triangulated::new(links, geometry_kind)));
        self.per_object_kind_statistics[geometry_kind].register_new_object();

        links.uid()
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_triangles(&mut self) -> GpuReadyTriangles {
        let bvh = build_serialized_bvh(&mut self.triangles);
        let triangles = serialize_batch(&self.triangles);
        GpuReadyTriangles::new(triangles, bvh)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized(&self, kind: DataKind) -> GpuReadySerializationBuffer {
        assert_ne!(kind, DataKind::TriangleMesh, "call 'evaluate_serialized_triangles' instead");
        self.serialize(kind)
    }

    #[must_use]
    pub(crate) fn count_of_a_kind(&self, kind: DataKind) -> usize {
        self.per_object_kind_statistics[kind as usize].object_count()
    }

    #[must_use]
    pub(crate) fn data_version(&self, kind: DataKind) -> Version {
        self.per_object_kind_statistics[kind as usize].data_version()
    }

    #[must_use]
    fn add_object<Constructor: FnOnce(ObjectUid) -> Box<dyn SceneObject>>(
        container: &mut HashMap<ObjectUid, Box<dyn SceneObject>>,
        uid_generator: &mut UidGenerator,
        statistics: &mut Vec<Statistics>,
        create_object: Constructor,
    ) -> ObjectUid {
        let uid = uid_generator.next();
        let object = create_object(uid);

        statistics[object.data_kind_uid()].register_new_object();
        container.insert(uid, object);

        uid
    }

    #[must_use]
    fn serialize(&self, desired_kind: DataKind) -> GpuReadySerializationBuffer { // TODO: we can reuse the buffer in case object count is the same
        let desired_kind = desired_kind as usize;
        let count = self.per_object_kind_statistics[desired_kind].object_count();
        assert!(count > 0, "gpu can't accept empty buffer");

        let sorted_of_a_kind: Vec<(u32, &dyn SceneObject)> =
        {
            let mut sorted = Vec::with_capacity(count);
            for (key, object) in self.objects.iter() {
                if object.data_kind_uid() == desired_kind {
                    sorted.push((key.0, object.as_ref()));
                }
            }
            debug_assert_eq!(sorted.len(), count);

            /*
            We can do without sorting, serializing in the loop above. But the stable
            order will make testing (especially automated tests) and debugging easier.
            */
            sorted.sort_by_key(|x| x.0);
            sorted
        };
        
        let quartets_per_object = sorted_of_a_kind[0].1.serialized_quartet_count();
        let mut result = GpuReadySerializationBuffer::new(count, quartets_per_object);
        for (_, object) in sorted_of_a_kind {
            object.serialize_into(&mut result);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::transform::{Affine, Transformation};
    use crate::objects::common_properties::Linkage;
    use crate::objects::material::Material;
    use crate::objects::material_index::MaterialIndex;
    use crate::objects::parallelogram::Parallelogram;
    use crate::objects::sdf::SdfBox;
    use crate::objects::sphere::Sphere;
    use crate::scene::container::{Container, DataKind};
    use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::utils::object_uid::ObjectUid;
    use cgmath::{EuclideanSpace, SquareMatrix, Zero};
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;

    #[must_use]
    fn make_test_mesh() -> (MeshWarehouse, WarehouseSlot) {
        let mut warehouse = MeshWarehouse::new();
        let dummy_mesh_path = Path::new("assets").join("tests/mesh/cube.obj");
        let slot = warehouse.load(&dummy_mesh_path).unwrap();
        (warehouse, slot)
    }
    
    #[test]
    fn test_set_material() {
        let system_under_test = Rc::new(RefCell::new(Container::new()));
        
        let material_one = system_under_test.borrow_mut().materials().add(&Material::default());
        let material_two = system_under_test.borrow_mut().materials().add(&Material::default());

        let assert_material_changed = |from: MaterialIndex, to: MaterialIndex, victim: ObjectUid| {
            assert_eq!(system_under_test.borrow().material_of(victim), from);
            system_under_test.borrow_mut().set_material(victim, to);
            assert_eq!(system_under_test.borrow().material_of(victim), to);
        };
        
        let parallelogram = system_under_test.borrow_mut().add_parallelogram(Point::origin(), Vector::zero(), Vector::zero(), material_one);
        let version_before = system_under_test.borrow().data_version(DataKind::Parallelogram);
        assert_material_changed(material_one, material_two, parallelogram);
        assert_ne!(system_under_test.borrow().data_version(DataKind::Parallelogram), version_before);
        
        let sphere = system_under_test.borrow_mut().add_sphere(Point::origin(), 1.0, material_one);
        let version_before = system_under_test.borrow().data_version(DataKind::Sphere);
        assert_material_changed(material_one, material_two, sphere);
        assert_ne!(system_under_test.borrow().data_version(DataKind::Sphere), version_before);
        
        assert_material_changed(material_two, material_one, parallelogram);
        
        let sdf = system_under_test.borrow_mut().add_sdf_box(&Affine::identity(), Vector::new(1.0,1.0,1.0), 0.0, material_one);
        let version_before = system_under_test.borrow().data_version(DataKind::Sdf);
        assert_material_changed(material_one, material_two, sdf);
        assert_ne!(system_under_test.borrow().data_version(DataKind::Sdf), version_before);
        
        assert_material_changed(material_one, material_two, parallelogram);
        assert_material_changed(material_two, material_one, sphere);

        let (mesh_warehouse, mesh_slot) = make_test_mesh();
        let mesh = system_under_test.borrow_mut().add_mesh(&mesh_warehouse, mesh_slot, &Transformation::identity(), material_one);
        let version_before = system_under_test.borrow().data_version(DataKind::TriangleMesh);
        assert_material_changed(material_one, material_two, mesh);
        assert_ne!(system_under_test.borrow().data_version(DataKind::TriangleMesh), version_before);

        assert_material_changed(material_two, material_one, parallelogram);
        assert_material_changed(material_one, material_two, sphere);
        assert_material_changed(material_two, material_one, sdf);
    }

    #[test]
    fn test_add_sdf() {
        let mut system_under_test = Container::new();

        const SDF_TO_ADD: u32 = 5;

        let material 
            = Material::default()
                .with_albedo(1.0, 0.0, 0.0)
                .with_emission(3.0, 2.0, 7.0)
                .with_specular(2.0, 4.6, 8.4)
                .with_roughness(-3.0);
        
        let expected_material = system_under_test.materials().add(&material);
        let expected_transform = Affine::identity();
        let expected_size = Vector::new(5.0, 7.0, 3.0);
        let expected_corners_radius = 0.7;
        
        let mut expected_serialized = GpuReadySerializationBuffer::new(SDF_TO_ADD as usize, <SdfBox as GpuSerializationSize>::SERIALIZED_QUARTET_COUNT);
        for i in 0_u32..SDF_TO_ADD
        {
            {
                let linkage = Linkage::new(ObjectUid(i+1), expected_material);
                let expected_sdf = SdfBox::new(Affine::identity(), Vector::new(5.0, 7.0, 3.0), expected_corners_radius, linkage);
                expected_sdf.serialize_into(&mut expected_serialized);
            }
            assert_eq!(system_under_test.count_of_a_kind(DataKind::Sdf), i as usize);
            {
                let data_version_before_addition = system_under_test.data_version(DataKind::Sdf);
                system_under_test.add_sdf_box(&expected_transform, expected_size, expected_corners_radius, expected_material);
                let data_version_after_addition = system_under_test.data_version(DataKind::Sdf);
                assert_ne!(data_version_before_addition, data_version_after_addition);
            }
        }

        let actual_serialized = system_under_test.evaluate_serialized(DataKind::Sdf);

        assert_eq!(actual_serialized.backend(), expected_serialized.backend());
    }
    
    #[test]
    fn test_add_parallelogram() {
        let mut system_under_test = Container::new();

        const PARALLELOGRAM_TO_ADD: u32 = 4;

        let expected_material = system_under_test.materials().add(&Material::default().with_albedo(1.0, 0.0, 0.0));
        let expected_origin = Point::new(1.0, 2.0, 3.0);
        let expected_x = Vector::new(3.0, 5.0, 7.0);
        let expected_y = Vector::new(4.0, 6.0, 8.0);

        let mut expected_serialized = GpuReadySerializationBuffer::new(PARALLELOGRAM_TO_ADD as usize, Parallelogram::SERIALIZED_QUARTET_COUNT);
        for i in 0_u32..PARALLELOGRAM_TO_ADD
        {
            {
                let linkage = Linkage::new(ObjectUid(i+1), expected_material);
                let expected_parallelogram = Parallelogram::new(expected_origin, expected_x, expected_y, linkage);
                expected_parallelogram.serialize_into(&mut expected_serialized);
            }
            assert_eq!(system_under_test.count_of_a_kind(DataKind::Parallelogram), i as usize);
            {
                let data_version_before_addition = system_under_test.data_version(DataKind::Parallelogram);
                system_under_test.add_parallelogram(expected_origin, expected_x, expected_y, expected_material);
                let data_version_after_addition = system_under_test.data_version(DataKind::Parallelogram);
                assert_ne!(data_version_before_addition, data_version_after_addition);
            }
        }

        let actual_serialized = system_under_test.evaluate_serialized(DataKind::Parallelogram);

        assert_eq!(actual_serialized.backend(), expected_serialized.backend());
    }
    
    #[test]
    fn test_add_sphere() {
        let mut system_under_test = Container::new();

        let material = system_under_test.materials().add(&Material::default().with_albedo(1.0, 0.0, 0.0));
        
        let expected_sphere_center = Point::new(1.0, 2.0, 3.0);
        let expected_sphere_radius = 1.5;

        const SPHERES_TO_ADD: u32 = 3;
        
        let mut expected_serialized = GpuReadySerializationBuffer::new(SPHERES_TO_ADD as usize, Sphere::SERIALIZED_QUARTET_COUNT);
        for i in 0_u32..SPHERES_TO_ADD
        {
            {
                let linkage = Linkage::new(ObjectUid(i+1), material);
                let expected_sphere = Sphere::new(expected_sphere_center, expected_sphere_radius, linkage);
                expected_sphere.serialize_into(&mut expected_serialized);
            }
            assert_eq!(system_under_test.count_of_a_kind(DataKind::Sphere), i as usize);
            {
                let data_version_before_addition = system_under_test.data_version(DataKind::Sphere);
                system_under_test.add_sphere(expected_sphere_center, expected_sphere_radius, material);
                let data_version_after_addition = system_under_test.data_version(DataKind::Sphere);
                assert_ne!(data_version_before_addition, data_version_after_addition);
            }
        }

        let actual_serialized = system_under_test.evaluate_serialized(DataKind::Sphere);

        assert_eq!(actual_serialized.backend(), expected_serialized.backend());
    }
}
