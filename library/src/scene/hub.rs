use crate::animation::time_tracker::TimeTracker;
use crate::container::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::container::visual_objects::VisualObjects;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::objects::common_properties::ObjectUid;
use crate::objects::material_index::MaterialIndex;
use crate::sdf::framework::named_sdf::UniqueSdfClassName;
use std::io::Error;
use std::path::Path;

pub struct Hub {
    container: VisualObjects,
    time_tracker: TimeTracker,
}

impl Hub {
    #[must_use]
    pub(crate) fn new(container: VisualObjects) -> Self {
        Self {
            container,
            time_tracker: TimeTracker::new(),
        }
    }

    #[must_use]
    pub(crate) fn container(&self) -> &VisualObjects {
        &self.container
    }

    #[must_use]
    pub fn animator(&self) -> &TimeTracker {
        &self.time_tracker
    }

    #[must_use]
    pub fn animator_mutable(&mut self) -> &mut TimeTracker {
        &mut self.time_tracker
    }

    pub fn update_time(&mut self) {
        self.time_tracker.update_time();
    }
    
    pub fn clear_objects(&mut self) {
        self.container.clear_objects();
        self.time_tracker.clear();
    }

    pub fn add_sdf_with_ray_march_fix(&mut self, location: &Affine, ray_marching_step_scale: f64, class_uid: &UniqueSdfClassName, material: MaterialIndex) -> ObjectUid {
        let added = self.container.add_sdf(location, ray_marching_step_scale, class_uid, material);
        self.time_tracker.track(added, &self.container.morphable());
        added
    }
    
    pub fn add_sdf(&mut self, location: &Affine, class_uid: &UniqueSdfClassName, material: MaterialIndex) -> ObjectUid {
        const RAY_MARCHING_STEP_ID_SCALE: f64 = 1.0;
        self.add_sdf_with_ray_march_fix(location, RAY_MARCHING_STEP_ID_SCALE, class_uid, material)
    }
    
    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ObjectUid {
        self.container.add_parallelogram(origin, local_x, local_y, material)
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> ObjectUid {
        self.container.add_mesh(source, slot, transformation, material)
    }
    
    pub fn delete(&mut self, target: ObjectUid) {
        self.container.delete(target);
        self.time_tracker.forget(target, &self.container.morphable());
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
