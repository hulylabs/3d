use crate::material::material::MaterialProperties;
use crate::material::material_index::MaterialIndex;
use crate::material::procedural_textures::ProceduralTextures;
use crate::utils::version::Version;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;
use crate::shader::code::ShaderCode;

pub struct MaterialsWarehouse {
    materials: Vec<MaterialProperties>,
    procedural_textures: ProceduralTextures,
    materials_version: Version,
}

impl MaterialsWarehouse {
    #[must_use]
    pub(crate) fn new(procedural_textures: ProceduralTextures) -> Self {
        Self { materials: Vec::new(), procedural_textures, materials_version: Version(0), }
    }
    
    #[must_use]
    pub fn add(&mut self, target: &MaterialProperties) -> MaterialIndex {
        self.materials.push(*target);
        self.materials_version += 1;
        MaterialIndex(self.materials.len() - 1)
    }

    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.materials.len()
    }

    #[must_use]
    pub(crate) fn data_version(&self) -> Version {
        self.materials_version
    }

    #[must_use]
    pub(crate) fn serialize(&self) -> GpuReadySerializationBuffer {
        serialize_batch(&self.materials)
    }
    
    #[must_use]
    pub(crate) fn procedural_textures_code(&self) -> ShaderCode {
        self.procedural_textures.generate_gpu_code()
    }
}

impl Default for MaterialsWarehouse {
    #[must_use]
    fn default() -> Self {
        Self::new(ProceduralTextures::new(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_version_materials() {
        let mut system_under_test = MaterialsWarehouse::default();

        let version_before = system_under_test.data_version();
        let _ = system_under_test.add(&MaterialProperties::default());
        assert_ne!(system_under_test.data_version(), version_before);

        let version_before = system_under_test.data_version();
        let _ = system_under_test.add(&MaterialProperties::default());
        assert_ne!(system_under_test.data_version(), version_before);
    }

    #[test]
    fn test_add_material() {
        let mut system_under_test = MaterialsWarehouse::default();

        let dummy_material = system_under_test.add(&MaterialProperties::default());
        assert_eq!(system_under_test.count(), 1);

        let another_material = system_under_test.add(&MaterialProperties::default().with_albedo(1.0, 0.0, 0.0));
        assert_eq!(system_under_test.count(), 2);
        assert_ne!(dummy_material, another_material);
    }
}