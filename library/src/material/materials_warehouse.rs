use crate::material::material_index::MaterialIndex;
use crate::material::material_properties::MaterialProperties;
use crate::material::procedural_textures::ProceduralTextures;
use crate::material::texture_reference::TextureReference;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;
use crate::shader::code::ShaderCode;
use crate::utils::version::Version;

pub struct MaterialsWarehouse {
    materials: Vec<MaterialProperties>,
    procedural_textures: Option<ProceduralTextures>,
    materials_version: Version,
}

impl MaterialsWarehouse {
    #[must_use]
    pub(crate) fn new(procedural_textures: Option<ProceduralTextures>) -> Self {
        Self {
            materials: Vec::new(),
            procedural_textures,
            materials_version: Version(0),
        }
    }

    #[must_use]
    pub(crate) fn animated(&self, index: MaterialIndex) -> bool {
        let albedo_texture = self.materials[index.0].albedo_texture();
        if let TextureReference::Procedural(id) = albedo_texture {
            if let Some(textures) = &self.procedural_textures {
                return textures.animated(id);
            }
        }
        false
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
        if let Some(procedural_textures) = &self.procedural_textures {
            procedural_textures.generate_gpu_code()
        } else {
            ProceduralTextures::make_dummy_selection_function()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::material::procedural_texture_index::ProceduralTextureUid;
    use crate::material::texture_procedural_3d::TextureProcedural3D;
    use crate::shader::code::FunctionBody;
    use crate::shader::conventions;
    use super::*;

    #[test]
    fn test_data_version_materials() {
        let mut system_under_test = MaterialsWarehouse::new(None);

        let version_before = system_under_test.data_version();
        let _ = system_under_test.add(&MaterialProperties::default());
        assert_ne!(system_under_test.data_version(), version_before);

        let version_before = system_under_test.data_version();
        let _ = system_under_test.add(&MaterialProperties::default());
        assert_ne!(system_under_test.data_version(), version_before);
    }

    #[test]
    fn test_add_material() {
        let mut system_under_test = MaterialsWarehouse::new(None);

        let dummy_material = system_under_test.add(&MaterialProperties::default());
        assert_eq!(system_under_test.count(), 1);

        let another_material = system_under_test.add(&MaterialProperties::default().with_albedo(1.0, 0.0, 0.0));
        assert_eq!(system_under_test.count(), 2);
        assert_ne!(dummy_material, another_material);
    }

    #[test]
    fn test_animated_false() {
        let texture_body = "return vec3f(0.0, 0.0, 0.0);\n".to_string();
        let (texture_uid, mut system_under_test) = make_warehouse_with_a_texture(texture_body);

        let material_without_texture = system_under_test.add(&MaterialProperties::default());
        assert_eq!(system_under_test.animated(material_without_texture), false);

        let material_with_texture = system_under_test.add(&MaterialProperties::default()
            .with_albedo_texture(TextureReference::Procedural(texture_uid)));
        assert_eq!(system_under_test.animated(material_with_texture), false);
    }
    
    #[test]
    fn test_animated_true() {
        let texture_body = format!("return vec3f({}, 0.0, 0.0);\n", conventions::PARAMETER_NAME_THE_TIME);
        let (texture_uid, mut system_under_test) = make_warehouse_with_a_texture(texture_body);
        
        let material_with_texture = system_under_test.add(&MaterialProperties::default()
            .with_albedo_texture(TextureReference::Procedural(texture_uid)));
        assert!(system_under_test.animated(material_with_texture));
    }

    #[must_use]
    fn make_warehouse_with_a_texture(texture_body: String) -> (ProceduralTextureUid, MaterialsWarehouse) {
        let mut textures = ProceduralTextures::new(None);
        let texture_code = texture_body.to_string();
        let texture = TextureProcedural3D::from_simple_body(ShaderCode::<FunctionBody>::new(texture_code));
        let texture_uid = textures.add(texture, None);

        (texture_uid, MaterialsWarehouse::new(Some(textures)))
    }
}
