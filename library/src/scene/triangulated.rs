﻿use crate::objects::common_properties::Linkage;
use crate::objects::material_index::MaterialIndex;
use crate::scene::scene_object::{SceneEnvironment, SceneObject};
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(super) struct Triangulated {
    links: Linkage,
    geometry_kind: usize,
}

impl Triangulated {
    #[must_use]
    pub(super) fn new(links: Linkage, geometry_kind: usize,) -> Self {
        Self { links, geometry_kind, }
    }
}

impl SceneObject for Triangulated {
    fn material(&self) -> MaterialIndex {
        self.links.material_index()
    }

    fn set_material(&mut self, new_material: MaterialIndex, environment: &mut SceneEnvironment) {
        for triangle in environment {
            if triangle.host() == self.links.uid() {
                triangle.set_material(new_material)
            }
        }
        self.links.set_material_index(new_material);
    }

    fn data_kind_uid(&self) -> usize {
        self.geometry_kind
    }
    
    fn serialized_quartet_count(&self) -> usize {
        0
    }

    fn serialize_into(&self, _: &mut GpuReadySerializationBuffer) {
        debug_assert!(false, "this placeholder object serialization should not be called");
    }
}