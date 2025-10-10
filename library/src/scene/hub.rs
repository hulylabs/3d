use crate::animation::time_tracker::TimeTracker;
use crate::container::mesh_warehouse::{MeshWarehouse, WarehouseSlot};
use crate::container::visual_objects::VisualObjects;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::{Affine, Transformation};
use crate::geometry::utils::is_affine;
use crate::material::material_index::MaterialIndex;
use crate::objects::common_properties::ObjectUid;
use crate::objects::sdf_instance::SdfPayload;
use crate::sdf::framework::named_sdf::UniqueSdfClassName;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::text::scene_strings::SceneStrings;
use crate::utils::bitmap_utils::BitmapSize;
use more_asserts::assert_gt;
use std::cell::RefCell;
use std::io::Error;
use std::path::Path;
use std::rc::Rc;
use crate::text::string_object_uid::StringObjectUid;

pub struct Hub {
    container: Rc<RefCell<VisualObjects>>,
    strings: Option<SceneStrings>,
    time_tracker: TimeTracker,
}

impl Hub {
    const EMPTY_SDF_PAYLOAD: SdfPayload = SdfPayload(0);

    #[must_use]
    pub(crate) fn new(container: Rc<RefCell<VisualObjects>>, strings: Option<SceneStrings>) -> Self {
        Self {
            container,
            strings,
            time_tracker: TimeTracker::new(),
        }
    }

    #[must_use]
    pub(crate) fn container(&self) -> Rc<RefCell<VisualObjects>> {
        self.container.clone()
    }

    #[must_use]
    pub(crate) fn any_objects_have_animated_texture(&self) -> bool {
        self.container.borrow().any_object_has_animated_texture()
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
        self.container.borrow_mut().clear_objects();
        self.time_tracker.clear();
    }
    
    pub fn add_text(&mut self, text: &str, location: &Affine, material: MaterialIndex) -> StringObjectUid {
        if let Some(strings) = &mut self.strings {
            let string = strings.add(text, location, material);
            let sdfs_or_none = strings.get_spares_of(&string);
            if let Some(sdfs) = sdfs_or_none {
                self.time_tracker.track(sdfs, &self.container.borrow_mut().morphable());
            }
            string
        } else {
            StringObjectUid::default()
        }
    }

    pub fn add_sdf_with_ray_march_fix(&mut self, location: &Affine, ray_marching_step_scale: f64, class_uid: &UniqueSdfClassName, material: MaterialIndex) -> ObjectUid {
        assert!(is_affine(location), "projection matrices are not supported");
        assert_gt!(ray_marching_step_scale, 0.0);
        let mut container = self.container.borrow_mut();
        let added = container.add_sdf(location, ray_marching_step_scale, class_uid, material, Self::EMPTY_SDF_PAYLOAD);
        self.time_tracker.track(&[added], &container.morphable());
        added
    }
    
    pub fn add_sdf(&mut self, location: &Affine, class_uid: &UniqueSdfClassName, material: MaterialIndex) -> ObjectUid {
        const RAY_MARCHING_STEP_ID_SCALE: f64 = 1.0;
        self.add_sdf_with_ray_march_fix(location, RAY_MARCHING_STEP_ID_SCALE, class_uid, material)
    }
    
    pub fn add_parallelogram(&mut self, origin: Point, local_x: Vector, local_y: Vector, material: MaterialIndex) -> ObjectUid {
        self.container.borrow_mut().add_parallelogram(origin, local_x, local_y, material)
    }

    pub fn add_mesh(&mut self, source: &MeshWarehouse, slot: WarehouseSlot, transformation: &Transformation, material: MaterialIndex) -> ObjectUid {
        self.container.borrow_mut().add_mesh(source, slot, transformation, material)
    }
    
    pub fn delete(&mut self, target: ObjectUid) {
        let mut container = self.container.borrow_mut();
        container.delete(target);
        self.time_tracker.forget(target, &container.morphable());
    }

    pub fn dump_scene_bvh(&self, destination: impl AsRef<Path>) -> Result<(), Error> {
        self.container.borrow_mut().dump_scene_bvh(destination)
    }

    pub fn set_material(&mut self, victim: ObjectUid, material: MaterialIndex) {
        self.container.borrow_mut().set_material(victim, material)
    }

    #[must_use]
    pub fn material_of(&self, victim: ObjectUid) -> MaterialIndex {
        self.container.borrow_mut().material_of(victim)
    }

    #[must_use]
    pub(crate) fn glyph_atlas_size(&self) -> BitmapSize {
        if let Some(strings) = &self.strings {
            strings.glyph_atlas_size()
        } else {
            BitmapSize::new(1, 1)
        }
    }

    #[must_use]
    pub(crate) fn glyph_atlas_layout(&self) -> GpuReadySerializationBuffer {
        if let Some(strings) = &self.strings {
            strings.glyph_atlas_layout()
        } else {
            SceneStrings::make_empty_atlas_layout_buffer()
        }
    }

    pub(crate) fn access_glyphs_atlas_data(&self, delegate: impl FnOnce(&[u8])) {
        if let Some(strings) = &self.strings {
            delegate(strings.glyph_atlas_bitmap());
        }
    }
}
