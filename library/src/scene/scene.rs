use std::io::Error;
use std::path::Path;
use crate::animation::animator::Animator;
use crate::container::container::Container;
use crate::container::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::objects::common_properties::ObjectUid;
use crate::objects::material_index::MaterialIndex;
use crate::sdf::named_sdf::UniqueSdfClassName;

pub struct Scene {
    container: Container,
    animator: Animator,
}

impl Scene {
    #[must_use]
    pub(crate) fn new(container: Container) -> Self {
        Self {
            container,
            animator: Animator::new(),
        }
    }

    #[must_use]
    pub(crate) fn container(&self) -> &Container {
        &self.container
    }

    #[must_use]
    pub fn animator(&mut self) -> &mut Animator {
        &mut self.animator
    }
    
    pub fn clear_objects(&mut self) {
        self.container.clear_objects();
        self.animator.clear_objects();
    }

    pub fn add_sdf(&mut self, location: &Affine, class_uid: &UniqueSdfClassName, material: MaterialIndex) -> ObjectUid {
        self.container.add_sdf(location, class_uid, material)
    }
    
    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ObjectUid {
        self.container.add_parallelogram(origin, local_x, local_y, material)
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> ObjectUid {
        self.container.add_mesh(source, slot, transformation, material)
    }
    
    pub fn delete(&mut self, target: ObjectUid) {
        self.animator.end(target);
        self.container.delete(target);
    }

    pub fn dump_scene_bvh(&self, destination: impl AsRef<Path>) -> Result<(), Error> {
        self.container.dump_scene_bvh(destination)
    }

    pub fn set_material(&mut self, victim: ObjectUid, material: MaterialIndex) {
        self.container.set_material(victim, material)
    }

    #[must_use]
    pub fn material_of(&self, victim: ObjectUid) -> MaterialIndex {
        self.container.material_of(victim)
    }
}
