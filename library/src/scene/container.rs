use crate::bvh::builder::build_serialized_bvh;
use crate::bvh::proxy::SceneObjectProxy;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::objects::common_properties::Linkage;
use crate::objects::material_index::MaterialIndex;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::SdfInstance;
use crate::objects::sdf_class_index::SdfClassIndex;
use crate::objects::triangle::Triangle;
use crate::scene::bvh_proxies::{proxy_of_sdf, SceneObjects};
use crate::scene::materials_warehouse::MaterialsWarehouse;
use crate::scene::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::scene::monolithic::Monolithic;
use crate::scene::scene_object::SceneObject;
use crate::scene::sdf_warehouse::SdfWarehouse;
use crate::scene::statistics::Statistics;
use crate::scene::triangulated::Triangulated;
use crate::scene::version::Version;
use crate::sdf::code_generator::SdfRegistrator;
use crate::sdf::named_sdf::UniqueSdfClassName;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;
use crate::utils::object_uid::ObjectUid;
use crate::utils::remove_with_reorder::remove_with_reorder;
use crate::utils::uid_generator::UidGenerator;
use cgmath::SquareMatrix;
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
    sdf_prototypes: SdfWarehouse,
    
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
            sdf_prototypes: SdfWarehouse::new(sdf_classes),
            uid_generator: UidGenerator::new(),
        }
    }

    #[must_use]
    pub(crate) fn append_sdf_handling_code(&self, base_code: &str) -> String {
        format!("{}\n{}", base_code, self.sdf_prototypes.sdf_classes_code())
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
                0,
                Affine::identity(),
            ))
        })
    }

    pub fn add_sdf(&mut self, location: &Affine, class_uid: &UniqueSdfClassName, material: MaterialIndex) -> ObjectUid{
        let index = self.sdf_prototypes.properties_for_name(class_uid).unwrap_or_else(|| panic!("registration for the '{}' sdf has not been found", class_uid));
        Self::add_object(&mut self.objects, &mut self.uid_generator, &mut self.per_object_kind_statistics, |uid| {
            Box::new(Monolithic::new(
                DataKind::Sdf as usize,
                Box::new(SdfInstance::new(*location, *index, Linkage::new(uid, material))),
                index.0,
                *location,
            ))
        })
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> ObjectUid {
        let links = Linkage::new(self.uid_generator.next(), material);

        let instance = source.instantiate(slot, transformation, links,);
        instance.put_triangles_into(&mut self.triangles);

        let geometry_kind = DataKind::TriangleMesh as usize;
        self.objects.insert(links.uid(), Box::new(Triangulated::new(links, geometry_kind, 0, *transformation.forward())));
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
        if self.objects.is_empty() {
            return;
        }
        
        for object in self.objects.keys() {
            self.uid_generator.put_back(*object);
        }
        for statistics in self.per_object_kind_statistics.iter_mut() {
            statistics.clear_objects();
        }
        self.objects.clear();
        self.triangles.clear();
    }
    
    #[must_use]
    pub(crate) fn evaluate_serialized_triangles(&self) -> GpuReadySerializationBuffer {
        assert!(self.triangles.len() > 0, "gpu can't accept empty buffer");
        serialize_batch(&self.triangles)
    }

    #[must_use]
    pub(crate) fn evaluate_serialized_bvh(&self) -> GpuReadySerializationBuffer {
        assert!(self.bvh_object_count() > 0, "gpu can't accept empty buffer");
        
        let mut objects_to_tree: Vec<SceneObjectProxy> = Vec::with_capacity(self.bvh_object_count());

        self.triangles.make_proxies(&mut objects_to_tree);

        const SDF_KIND: usize = DataKind::Sdf as usize;
        let sdf_count = self.per_object_kind_statistics[SDF_KIND].object_count();
        if sdf_count > 0 {
            let sorted_of_a_kind = self.sorted_of_a_kind(SDF_KIND, sdf_count);
            for (index, sdf) in sorted_of_a_kind.iter().enumerate() {
                let class_index = sdf.entity.payload();
                let class_aabb = self.sdf_prototypes.aabb_from_index(SdfClassIndex(class_index));
                let instance_aabb = class_aabb.transform(sdf.entity.transformation());
                objects_to_tree.push(proxy_of_sdf(index, instance_aabb));
            }
        }

        build_serialized_bvh(&mut objects_to_tree)
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
    pub(crate) fn triangles_count(&self) -> usize {
        self.triangles.len()
    }

    #[must_use]
    pub(crate) fn bvh_inhabited(&self) -> bool {
        self.bvh_object_count() > 0
    }

    #[must_use]
    pub(crate) fn data_version(&self, kind: DataKind) -> Version {
        self.per_object_kind_statistics[kind as usize].data_version()
    }

    #[must_use]
    fn bvh_object_count(&self) -> usize {
        self.triangles.len() + self.count_of_a_kind(DataKind::Sdf)
    }

    #[must_use]
    fn add_object<Constructor: FnOnce(ObjectUid) -> Box<dyn SceneObject>>(
        container: &mut HashMap<ObjectUid, Box<dyn SceneObject>>,
        uid_generator: &mut UidGenerator,
        statistics: &mut [Statistics],
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
        let sorted_of_a_kind = self.sorted_of_a_kind(desired_kind, count);
        
        let quartets_per_object = sorted_of_a_kind[0].entity.serialized_quartet_count();
        let mut result = GpuReadySerializationBuffer::new(count, quartets_per_object);
        for object in sorted_of_a_kind {
            object.entity.serialize_into(&mut result);
        }

        result
    }
    
    fn sorted_of_a_kind(&self, desired_kind: usize, expected_count: usize) -> Vec<IdentifiedObject> {
        let mut sorted = Vec::with_capacity(expected_count);
        for (key, object) in self.objects.iter() {
            if object.data_kind_uid() == desired_kind {
                sorted.push(IdentifiedObject{ id: *key, entity: object.as_ref() });
            }
        }
        debug_assert_eq!(sorted.len(), expected_count);

        sorted.sort_by_key(|x| x.id.0);
        sorted
    }
}

struct IdentifiedObject<'a> {
    id: ObjectUid,
    entity: &'a dyn SceneObject,
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
    use crate::scene::version::Version;
    use crate::sdf::code_generator::SdfRegistrator;
    use crate::sdf::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::sdf_sphere::SdfSphere;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::utils::tests::assert_utils::tests::assert_all_not_equal;
    use crate::utils::object_uid::ObjectUid;
    use cgmath::{EuclideanSpace, SquareMatrix, Zero};
    use std::cell::RefCell;
    use std::io::Write;
    use std::path::Path;
    use std::rc::Rc;
    use strum::{EnumCount, IntoEnumIterator};
    use tempfile::NamedTempFile;

    #[must_use]
    fn make_test_mesh() -> (MeshWarehouse, WarehouseSlot) {
        let mut warehouse = MeshWarehouse::new();
        let dummy_mesh_path = Path::new("assets").join("tests/mesh/cube.obj");
        let slot = warehouse.load(&dummy_mesh_path).unwrap();
        (warehouse, slot)
    }

    #[must_use]
    fn make_single_sdf_sphere() -> (UniqueSdfClassName, SdfRegistrator) {
        let mut sdf_classes = SdfRegistrator::default();
        let sphere_sdf_name = UniqueSdfClassName::new("identity_sphere".to_string());
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
        let mut system_under_test = make_empty_container();

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
        let mut system_under_test = make_empty_container();

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
        let serialized_triangles = system_under_test.evaluate_serialized_triangles();
        assert_eq!(serialized_triangles.total_slots_count(), expected_mesh_count * triangles_in_a_cube);
    }

    #[test]
    fn test_delete_sdf() {
        let mut fixture = make_filled_container();
        
        let sdf_to_be_deleted = fixture.sdf;
        let sdf_to_be_kept = fixture.container.add_sdf(&Affine::identity(), &fixture.sdf_name, fixture.dummy_material);

        fixture.container.delete(sdf_to_be_deleted);
        
        assert_eq!(fixture.container.count_of_a_kind(DataKind::Sdf), 1);
        assert_eq!(fixture.container.count_of_a_kind(DataKind::Parallelogram), 1);
        assert_eq!(fixture.container.count_of_a_kind(DataKind::TriangleMesh), 1);
        
        // check if expected objects are kept: there will be a panic, if we try to get material of an absent object 
        assert_eq!(fixture.container.material_of(fixture.parallelogram), fixture.dummy_material);
        assert_eq!(fixture.container.material_of(sdf_to_be_kept), fixture.dummy_material);
        assert_eq!(fixture.container.material_of(fixture.mesh), fixture.dummy_material);
    }

    #[test]
    fn test_clean() {
        let mut fixture = make_filled_container();

        let version_before = get_versions(&fixture.container);
        fixture.container.clear_objects();
        let version_after = get_versions(&fixture.container);

        assert_all_not_equal(version_before.as_slice(), version_after.as_slice());

        assert_is_empty(&fixture.container);
    }

    #[test]
    fn test_clear_already_cleared() {
        let mut system_under_test = make_empty_container();

        let version_before = get_versions(&system_under_test);
        system_under_test.clear_objects();
        let version_after = get_versions(&system_under_test);

        assert_eq!(version_before, version_after);
    }

    #[test]
    fn test_bvh_inhabited() {
        let mut fixture = make_filled_container();
        assert!(fixture.container.bvh_inhabited());
        
        fixture.container.delete(fixture.sdf);
        assert!(fixture.container.bvh_inhabited());

        fixture.container.delete(fixture.mesh);
        assert_eq!(false, fixture.container.bvh_inhabited());
    }

    #[test]
    fn test_empty_container() {
        let system_under_test = make_empty_container();
        
        assert_eq!(false, system_under_test.bvh_inhabited(), "empty container expected to have bvh without primitives");
        assert_is_empty(&system_under_test);
    }

    #[must_use]
    fn make_empty_container() -> Container {
        Container::new(SdfRegistrator::default())
    }

    struct FilledContainerFixture {
        container: Container,
        dummy_material: MaterialIndex,
        sdf: ObjectUid,
        sdf_name: UniqueSdfClassName,
        parallelogram: ObjectUid,
        mesh: ObjectUid,
    }

    #[must_use]
    fn make_filled_container() -> FilledContainerFixture {
        let (sdf_name, sdf_classes) = make_single_sdf_sphere();
        let mut container = Container::new(sdf_classes);

        let dummy_material = container.materials().add(&Material::default());
        let (mesh_id, meshes) = prepare_test_mesh();

        let sdf = container.add_sdf(&Affine::identity(), &sdf_name, dummy_material);
        let parallelogram = container.add_parallelogram(Point::origin(), Vector::unit_x(), Vector::unit_y(), dummy_material);
        let mesh = container.add_mesh(&meshes, mesh_id, &Transformation::identity(), dummy_material);

        FilledContainerFixture { container, dummy_material, sdf, sdf_name, parallelogram, mesh, }
    }

    #[must_use]
    fn get_versions(from: &Container) -> Vec<Version> {
        let mut result: Vec<Version> = Vec::with_capacity(DataKind::COUNT);
        for kind in DataKind::iter() {
            result.push(from.data_version(kind));
        }
        result
    }

    fn assert_is_empty(fixture: &Container) {
        assert_eq!(fixture.triangles_count(), 0);
        for kind in DataKind::iter() {
            assert_eq!(fixture.count_of_a_kind(kind), 0);
        }
        assert_eq!(false, fixture.bvh_inhabited());
    }

}
