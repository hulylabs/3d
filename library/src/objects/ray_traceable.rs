use crate::material::material_index::MaterialIndex;
use crate::serialization::serializable_for_gpu::GpuSerializable;

pub(crate) trait RayTraceable: GpuSerializable {
    fn material(&self) -> MaterialIndex;
    fn set_material(&mut self, material_index: MaterialIndex);
    
    fn serialized_quartet_count(&self) -> usize;
}