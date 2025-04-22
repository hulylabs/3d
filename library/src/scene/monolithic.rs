use crate::objects::material_index::MaterialIndex;
use crate::scene::scene_object::{SceneEnvironment, SceneObject};
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::objects::ray_traceable::RayTraceable;

pub(super) struct Monolithic {
    geometry_kind: usize,
    geometry: Box<dyn RayTraceable>,
}

impl Monolithic {
    #[must_use]
    pub(super) fn new(geometry_kind: usize, backend: Box<dyn RayTraceable>) -> Self {
        Self { geometry_kind, geometry: backend }
    }
}

impl SceneObject for Monolithic {
    #[must_use]
    fn material(&self) -> MaterialIndex {
        self.geometry.material()
    }
    fn set_material(&mut self, new_material: MaterialIndex, _environment: &mut SceneEnvironment) {
        self.geometry.set_material(new_material)
    }

    #[must_use]
    fn data_kind_uid(&self) -> usize {
        self.geometry_kind
    }

    #[must_use]
    fn serialized_quartet_count(&self) -> usize {
        self.geometry.serialized_quartet_count()
    }
    fn serialize_into(&self, buffer: &mut GpuReadySerializationBuffer) {
        self.geometry.serialize_into(buffer);
    }
}
