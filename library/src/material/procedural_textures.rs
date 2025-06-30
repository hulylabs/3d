use crate::material::procedural_texture_index::ProceduralTextureUid;
use crate::material::texture_procedural::TextureProcedural;
use crate::material::texture_shader_code::{write_texture_code, write_texture_selection, write_texture_selection_function_opening};
use crate::shader::code::{Generic, ShaderCode};
use crate::shader::function_name::FunctionName;
use std::collections::HashMap;
use std::fmt::Write;

pub struct ProceduralTextures {
    shared_procedure_textures_code: ShaderCode,
    textures: HashMap<FunctionName, IdentifiedTextureProcedural>,
}

impl Default for ProceduralTextures {
    #[must_use]
    fn default() -> Self {
        ProceduralTextures::new(None)
    }
}

struct IdentifiedTextureProcedural {
    texture: TextureProcedural,
    uid: ProceduralTextureUid,
}

impl ProceduralTextures {
    #[must_use]
    pub fn new(shared_procedure_textures_code: Option<ShaderCode>) -> Self {
        let shared_code = shared_procedure_textures_code.unwrap_or(ShaderCode::<Generic>::new(String::new()));
        Self {
            shared_procedure_textures_code: shared_code,
            textures: HashMap::new(),
        }
    }

    #[must_use]
    pub fn add(&mut self, name: FunctionName, target: TextureProcedural) -> ProceduralTextureUid {
        assert!(self.textures.get(&name).is_none());
        let uid = ProceduralTextureUid(self.textures.len() + 1);
        self.textures.insert(name, IdentifiedTextureProcedural { texture: target, uid, });
        uid
    }

    #[must_use]
    pub(crate) fn generate_gpu_code(&self) -> ShaderCode {
        let mut buffer: String = self.shared_procedure_textures_code.to_string();

        if buffer.len() > 0 {
            buffer.push('\n');
        }
        self.write_gpu_code(&mut buffer).expect("shader code formatting failed");

        ShaderCode::<Generic>::new(buffer)
    }

    fn write_gpu_code(&self, mut buffer: &mut String) -> anyhow::Result<()> {
        let mut sorted_by_index: Vec<(&FunctionName, &IdentifiedTextureProcedural)> = self.textures.iter().collect();
        sorted_by_index.sort_by_key(|(name, _)| &name.0);

        for (name, item) in sorted_by_index.iter() {
            let body = item.texture.function_body();
            write_texture_code(body, name, &mut buffer)?;
        }
        Self::write_selection_function(&mut sorted_by_index, &mut buffer)?;

        Ok(())
    }

    fn write_selection_function(variants: &mut Vec<(&FunctionName, &IdentifiedTextureProcedural)>, buffer: &mut String) -> anyhow::Result<()> {
        write_texture_selection_function_opening(buffer)?;
        writeln!(buffer, " {{")?;

        for variant in variants {
            write_texture_selection(variant.0, variant.1.uid, buffer)?;
        }

        write!(buffer, "return vec3f(0.0);\n}}\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use more_asserts::assert_gt;
    use super::*;
    use crate::shader::code::{FunctionBody, Generic, ShaderCode};
    use crate::shader::function_name::FunctionName;

    #[must_use]
    fn procedural_texture(body: &str) -> TextureProcedural {
        TextureProcedural::new(ShaderCode::<FunctionBody>::new(body.to_string()))
    }
    
    #[test]
    fn test_new_with_shared_code() {
        let expected_generated_code = "shared texture code".to_string();
        let shared_code = ShaderCode::<Generic>::new(expected_generated_code.clone());
        let system_under_test = ProceduralTextures::new(Some(shared_code));

        let generated_code = system_under_test.generate_gpu_code();
        assert_eq!(
            generated_code.to_string(),
            format!(
                "{}\nfn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32) -> vec3f {{\nreturn vec3f(0.0);\n}}\n",
                expected_generated_code
            )
        );
    }

    #[test]
    fn test_new_without_shared_code() {
        let system_under_test = ProceduralTextures::new(None);

        let generated_code = system_under_test.generate_gpu_code();

        assert_eq!(
            generated_code.to_string(),
            format!("fn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32) -> vec3f {{\nreturn vec3f(0.0);\n}}\n")
        );
    }

    #[test]
    fn test_add_single_texture() {
        let mut system_under_test = ProceduralTextures::new(None);
        let texture = procedural_texture("return vec3f(1.0, 0.0, 0.0);");
        let function_name = FunctionName("test_texture".to_string());

        let uid = system_under_test.add(function_name, texture);
        assert_gt!(uid.0, 0);
        let generated_code = system_under_test.generate_gpu_code();

        assert!(generated_code.as_str().contains(format!("if (texture_index == {})", uid).as_str()));
    }

    #[test]
    fn test_add_multiple_textures() {
        let mut system_under_test = ProceduralTextures::new(None);

        let first_uid = system_under_test.add(
            FunctionName("red_texture".to_string()),
            procedural_texture("return vec3f(1.0, 0.0, 0.0);"),
        );
        let second_uid = system_under_test.add(
            FunctionName("green_texture".to_string()),
            procedural_texture("return vec3f(0.0, 1.0, 0.0);"),
        );
        let third_uid = system_under_test.add(
            FunctionName("blue_texture".to_string()),
            procedural_texture("return vec3f(0.0, 0.0, 1.0);"),
        );

        assert_ne!(first_uid, second_uid);
        assert_ne!(second_uid, third_uid);
    }

    #[test]
    fn test_generate_gpu_code_multiple_textures() {
        let mut system_under_test = ProceduralTextures::new(None);
        let first_texture = procedural_texture("return vec3f(1.0, 0.0, 0.0);");
        let second_texture = procedural_texture("return vec3f(0.0, 1.0, 0.0);");
    
        let first_name = FunctionName("red_texture".to_string());
        let second_name = FunctionName("green_texture".to_string());
    
        let _ = system_under_test.add(first_name, first_texture);
        let _ = system_under_test.add(second_name, second_texture);
    
        let actual_code = system_under_test.generate_gpu_code();
        let expected_code = "fn green_texture(point: vec3f, normal: vec3f, time: f32)->vec3f{\nreturn vec3f(0.0, 1.0, 0.0);\n}\nfn red_texture(point: vec3f, normal: vec3f, time: f32)->vec3f{\nreturn vec3f(1.0, 0.0, 0.0);\n}\nfn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32) -> vec3f {\nif (texture_index == 1) { return green_texture(point,normal,time); }\nif (texture_index == 0) { return red_texture(point,normal,time); }\nreturn vec3f(0.0);\n}\n";
        
        assert_eq!(actual_code.to_string(), expected_code);
    }
    
    #[test]
    #[should_panic]
    fn test_add_with_same_function() {
        let mut system_under_test = ProceduralTextures::new(None);
        let texture = procedural_texture("return vec3f(3.0, 0.0, 0.0);");
    
        let function_name = FunctionName("test_texture".to_string());
    
        let _ = system_under_test.add(function_name.clone(), texture.clone());
        let _ = system_under_test.add(function_name, texture);
    }
    
    #[test]
    fn test_generate_gpu_code_multiple_calls_same_result() {
        let mut system_under_test = ProceduralTextures::new(None);
        let texture = procedural_texture("return vec3f(0.8, 0.2, 0.1);");
        let function_name = FunctionName("orange_texture".to_string());
    
        let _ = system_under_test.add(function_name, texture);
    
        let first = system_under_test.generate_gpu_code();
        let second = system_under_test.generate_gpu_code();
    
        assert_eq!(first, second);
    }
}
