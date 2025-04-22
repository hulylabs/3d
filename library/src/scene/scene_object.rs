use crate::objects::material_index::MaterialIndex;
use crate::objects::triangle::Triangle;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(super) type SceneEnvironment = Vec<Triangle>;

pub(super) trait SceneObject {
    #[must_use]
    fn material(&self) -> MaterialIndex;
    fn set_material(&mut self, new_material: MaterialIndex, environment: &mut SceneEnvironment);

    #[must_use]
    fn data_kind_uid(&self) -> usize;
    
    #[must_use]
    fn serialized_quartet_count(&self) -> usize;
    fn serialize_into(&self, buffer: &mut GpuReadySerializationBuffer);
}