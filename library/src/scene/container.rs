use crate::bvh::builder::build_serialized_bvh;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::objects::common_properties::Linkage;
use crate::objects::material_index::MaterialIndex;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::SdfInstance;
use crate::objects::triangle::Triangle;
use crate::scene::gpu_ready_triangles::GpuReadyTriangles;
use crate::scene::materials_warehouse::MaterialsWarehouse;
use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::scene::monolithic::Monolithic;
use crate::scene::scene_object::SceneObject;
use crate::sdf::code_generator::SdfRegistrator;
use crate::sdf::named_sdf::UniqueName;
use crate::scene::sdf_warehouse::SdfWarehouse;
use crate::scene::statistics::Statistics;
use crate::scene::triangulated::Triangulated;
use crate::scene::version::Version;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;
use crate::utils::object_uid::ObjectUid;
use crate::utils::remove_with_reorder::remove_with_reorder;
use crate::utils::uid_generator::UidGenerator;
use std::collections::HashMap;
use strum::EnumCount;
use strum_macros::{AsRefStr, Display, EnumCount, EnumIter};

#[derive(EnumIter, EnumCount, Display, AsRefStr, Copy, Clone, PartialEq, Debug)]
pub(crate) enum DataKind {
    Parallelogram,
    Sdf,
    TriangleMesh,
}

pub struct Container {
    per_object_kind_statistics: Vec<Statistics>,
    objects: HashMap<ObjectUid, Box<dyn SceneObject>>,
    triangles: Vec<Triangle>,
    
    materials: MaterialsWarehouse,
    sdfs: SdfWarehouse,
    
    uid_generator: UidGenerator,
}

impl Container {
    #[must_use]
    pub fn new(sdf_classes: SdfRegistrator) -> Self {
        let per_object_kind_statistics: Vec<Statistics> = vec![Statistics::default(); DataKind::COUNT];

        Self {
            per_object_kind_statistics,
            objects: HashMap::new(),
            triangles: Vec::new(),
            materials: MaterialsWarehouse::new(),
            sdfs: SdfWarehouse::new(sdf_classes),
            uid_generator: UidGenerator::new(),
        }
    }

    #[must_use]
    pub(crate) fn append_sdf_handling_code(&self, base_code: &str) -> String {
        format!("{}\n{}", base_code, self.sdfs.sdf_classes_code())
    }

    #[must_use]
    pub fn materials(&mut self) -> &mut MaterialsWarehouse {
        &mut self.materials
    }

    pub fn set_material(&mut self, victim: ObjectUid, material: MaterialIndex) {
        match self.objects.get_mut(&victim) {
            Some(object) => {
                if object.material() != material {
                    object.set_material(material, &mut self.triangles);
                    self.per_object_kind_statistics[object.data_kind_uid()].register_object_mutation();   
                }
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

    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ObjectUid {
        Self::add_object(&mut self.objects, &mut self.uid_generator, &mut self.per_object_kind_statistics, |uid| {
            Box::new(Monolithic::new(
                DataKind::Parallelogram as usize,
                Box::new(Parallelogram::new(origin, local_x, local_y, Linkage::new(uid, material))),
            ))
        })
    }

    pub fn add_sdf(&mut self, location: &Affine, class_uid: &UniqueName, material: MaterialIndex) -> ObjectUid{
        let index_or_none = self.sdfs.index_for_name(class_uid);
        if let Some(index) = index_or_none {
            Self::add_object(&mut self.objects, &mut self.uid_generator, &mut self.per_object_kind_statistics, |uid| {
                Box::new(Monolithic::new(
                    DataKind::Sdf as usize,
                    Box::new(SdfInstance::new(*location, *index, Linkage::new(uid, material))),
                ))
            })
        } else {
            panic!("registration for the '{}' sdf has not been found", class_uid);
        }
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

    pub fn delete(&mut self, target: ObjectUid) {
        let removed_or_none = self.objects.remove(&target);
        if let Some(removed) = removed_or_none {
            self.per_object_kind_statistics[removed.data_kind_uid()].delete_object();
            self.uid_generator.put_back(target);
            
            if removed.data_kind_uid() == DataKind::TriangleMesh as usize {
                remove_with_reorder(&mut self.triangles, |triangle| triangle.host() == target);
            }
        }
    }
    
    pub fn clear_objects(&mut self) {
        self.triangles.clear();
        for object in self.objects.keys() {
            self.uid_generator.put_back(*object);
        }
        for statistics in self.per_object_kind_statistics.iter_mut() {
            statistics.clear_objects();
        }
        self.objects.clear();
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
    use crate::objects::sdf::SdfInstance;
    use crate::objects::sdf_class_index::SdfClassIndex;
    use crate::scene::container::{Container, DataKind};
    use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
    use crate::sdf::code_generator::SdfRegistrator;
    use crate::sdf::named_sdf::{NamedSdf, UniqueName};
    use crate::sdf::sdf_sphere::SdfSphere;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::utils::object_uid::ObjectUid;
    use cgmath::{EuclideanSpace, SquareMatrix, Zero};
    use std::cell::RefCell;
    use std::io::Write;
    use std::path::Path;
    use std::rc::Rc;
    use tempfile::NamedTempFile;

    #[must_use]
    fn make_test_mesh() -> (MeshWarehouse, WarehouseSlot) {
        let mut warehouse = MeshWarehouse::new();
        let dummy_mesh_path = Path::new("assets").join("tests/mesh/cube.obj");
        let slot = warehouse.load(&dummy_mesh_path).unwrap();
        (warehouse, slot)
    }

    #[must_use]
    fn make_single_sdf_sphere() -> (UniqueName, SdfRegistrator) {
        let mut sdf_classes = SdfRegistrator::new();
        let sphere_sdf_name = UniqueName::new("identity_sphere".to_string());
        sdf_classes.add(&NamedSdf::new(SdfSphere::new(1.0), sphere_sdf_name.clone()));

        (sphere_sdf_name, sdf_classes)
    }
    
    #[test]
    fn test_set_material() {
        let (sphere_sdf_name, sdf_classes) = make_single_sdf_sphere();
        let system_under_test = Rc::new(RefCell::new(Container::new(sdf_classes)));
        
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
        
        assert_material_changed(material_two, material_one, parallelogram);
        
        let sdf = system_under_test.borrow_mut().add_sdf(&Affine::identity(), &sphere_sdf_name, material_one);
        let version_before = system_under_test.borrow().data_version(DataKind::Sdf);
        assert_material_changed(material_one, material_two, sdf);
        assert_ne!(system_under_test.borrow().data_version(DataKind::Sdf), version_before);
        
        assert_material_changed(material_one, material_two, parallelogram);

        let (mesh_warehouse, mesh_slot) = make_test_mesh();
        let mesh = system_under_test.borrow_mut().add_mesh(&mesh_warehouse, mesh_slot, &Transformation::identity(), material_one);
        let version_before = system_under_test.borrow().data_version(DataKind::TriangleMesh);
        assert_material_changed(material_one, material_two, mesh);
        assert_ne!(system_under_test.borrow().data_version(DataKind::TriangleMesh), version_before);

        assert_material_changed(material_two, material_one, parallelogram);
        assert_material_changed(material_two, material_one, sdf);
    }

    #[test]
    fn test_add_sdf() {
        let (sphere_sdf_name, sdf_classes) = make_single_sdf_sphere();
        let mut system_under_test = Container::new(sdf_classes);

        const SDF_TO_ADD: u32 = 5;

        let material 
            = Material::default()
                .with_albedo(1.0, 0.0, 0.0)
                .with_emission(3.0, 2.0, 7.0)
                .with_specular(2.0, 4.6, 8.4)
                .with_roughness(-3.0);
        
        let expected_material = system_under_test.materials().add(&material);
        let expected_transform = Affine::identity();

        let mut expected_serialized = GpuReadySerializationBuffer::new(SDF_TO_ADD as usize, <SdfInstance as GpuSerializationSize>::SERIALIZED_QUARTET_COUNT);
        for i in 0_u32..SDF_TO_ADD
        {
            {
                let linkage = Linkage::new(ObjectUid(i+1), expected_material);
                let expected_sdf = SdfInstance::new(Affine::identity(), SdfClassIndex(0), linkage);
                expected_sdf.serialize_into(&mut expected_serialized);
            }
            assert_eq!(system_under_test.count_of_a_kind(DataKind::Sdf), i as usize);
            {
                let data_version_before_addition = system_under_test.data_version(DataKind::Sdf);
                system_under_test.add_sdf(&expected_transform, &sphere_sdf_name, expected_material);
                let data_version_after_addition = system_under_test.data_version(DataKind::Sdf);
                assert_ne!(data_version_before_addition, data_version_after_addition);
            }
        }

        let actual_serialized = system_under_test.evaluate_serialized(DataKind::Sdf);

        assert_eq!(actual_serialized.backend(), expected_serialized.backend());
    }
    
    #[test]
    fn test_add_parallelogram() {
        let mut system_under_test = Container::new(SdfRegistrator::new());

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
    
    #[must_use]
    fn prepare_test_mesh() -> (WarehouseSlot, MeshWarehouse) {
        let mut temp_file = NamedTempFile::new_in("./").expect("failed to create temp file");
        temp_file.write_all(CUBE_OBJ_FILE.as_bytes()).expect("failed to write cube data into the temp file");

        let mut warehouse = MeshWarehouse::new();
        let mesh_index = warehouse.load(temp_file.path()).unwrap();

        (mesh_index, warehouse)
    }

    #[test]
    fn test_delete_mesh() {
        let mut system_under_test = Container::new(SdfRegistrator::new());

        let (mesh, meshes) = prepare_test_mesh();
        let dummy_material = system_under_test.materials().add(&Material::default());
        
        let to_be_kept_one = system_under_test.add_mesh(&meshes, mesh, &Transformation::identity(), dummy_material);
        let to_be_deleted = system_under_test.add_mesh(&meshes, mesh, &Transformation::identity(), dummy_material);
        let to_be_kept_two = system_under_test.add_mesh(&meshes, mesh, &Transformation::identity(), dummy_material);
        let to_be_kept_three = system_under_test.add_mesh(&meshes, mesh, &Transformation::identity(), dummy_material);

        system_under_test.delete(to_be_deleted);

        let expected_mesh_count = 3;
        assert_eq!(system_under_test.count_of_a_kind(DataKind::TriangleMesh), expected_mesh_count);
        assert_eq!(system_under_test.material_of(to_be_kept_one), dummy_material);
        assert_eq!(system_under_test.material_of(to_be_kept_two), dummy_material);
        assert_eq!(system_under_test.material_of(to_be_kept_three), dummy_material);

        let triangles_in_a_cube = 12;
        let mut serialized_triangles = system_under_test.evaluate_serialized_triangles();
        assert_eq!(serialized_triangles.extract_geometry().total_slots_count(), expected_mesh_count * triangles_in_a_cube);
    }

    #[test]
    fn test_delete_sdf() {
        let (sphere_sdf_name, sdf_classes) = make_single_sdf_sphere();
        let mut system_under_test = Container::new(sdf_classes);

        let dummy_material = system_under_test.materials().add(&Material::default());
        let (mesh, meshes) = prepare_test_mesh();
        
        let to_be_deleted = system_under_test.add_sdf(&Affine::identity(), &sphere_sdf_name, dummy_material);
        let parallelogram_to_keep = system_under_test.add_parallelogram(Point::origin(), Vector::unit_x(), Vector::unit_y(), dummy_material);
        let sdf_to_keep = system_under_test.add_sdf(&Affine::identity(), &sphere_sdf_name, dummy_material);
        let mesh_to_keep = system_under_test.add_mesh(&meshes, mesh, &Transformation::identity(), dummy_material);
        
        system_under_test.delete(to_be_deleted);
        
        assert_eq!(system_under_test.count_of_a_kind(DataKind::Sdf), 1);
        assert_eq!(system_under_test.count_of_a_kind(DataKind::Parallelogram), 1);
        assert_eq!(system_under_test.count_of_a_kind(DataKind::TriangleMesh), 1);
        
        // check if expected objects are kept: there will be a panic, if we try to get material of an absent object 
        assert_eq!(system_under_test.material_of(parallelogram_to_keep), dummy_material);
        assert_eq!(system_under_test.material_of(sdf_to_keep), dummy_material);
        assert_eq!(system_under_test.material_of(mesh_to_keep), dummy_material);
    }
}
