use crate::objects::material::Material;
use crate::objects::material_index::MaterialIndex;
use crate::scene::version::Version;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;

pub struct MaterialsWarehouse {
    materials: Vec<Material>,
    materials_version: Version,
}

impl MaterialsWarehouse {
    #[must_use]
    pub fn add(&mut self, target: &Material) -> MaterialIndex {
        self.materials.push(*target);
        self.materials_version += 1;
        MaterialIndex(self.materials.len() - 1)
    }
    
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { materials: Vec::new(), materials_version: Version(0) }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_version_materials() {
        let mut system_under_test = MaterialsWarehouse::new();

        let version_before = system_under_test.data_version();
        let _ = system_under_test.add(&Material::default());
        assert_ne!(system_under_test.data_version(), version_before);

        let version_before = system_under_test.data_version();
        let _ = system_under_test.add(&Material::default());
        assert_ne!(system_under_test.data_version(), version_before);
    }

    #[test]
    fn test_add_material() {
        let mut system_under_test = MaterialsWarehouse::new();

        let dummy_material = system_under_test.add(&Material::default());
        assert_eq!(system_under_test.count(), 1);

        let another_material = system_under_test.add(&Material::default().with_albedo(1.0, 0.0, 0.0));
        assert_eq!(system_under_test.count(), 2);
        assert_ne!(dummy_material, another_material);
    }
}