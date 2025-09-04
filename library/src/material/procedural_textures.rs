use crate::material::procedural_texture_index::ProceduralTextureUid;
use crate::material::texture_procedural_3d::TextureProcedural3D;
use crate::material::texture_shader_code::{write_texture_3d_code, write_texture_3d_selection, write_texture_3d_selection_function_opening};
use crate::shader::code::{Generic, ShaderCode};
use crate::shader::function_name::FunctionName;
use crate::shader::function_name_generator::FunctionNameGenerator;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;
use std::rc::Rc;
use crate::material::triplanar_mapper::TriplanarMapper;

pub struct ProceduralTextures {
    shared_procedure_textures_code: ShaderCode,
    textures: HashMap<ProceduralTextureUid, NamedTextureProcedural>,
    names_generator: Rc<RefCell<FunctionNameGenerator>>,
}

struct NamedTextureProcedural {
    texture: TextureProcedural3D,
    name: FunctionName,
}

impl ProceduralTextures {
    #[must_use]
    pub fn new(shared_procedure_textures_code: Option<ShaderCode>) -> Self {
        let shared_code = shared_procedure_textures_code.unwrap_or(ShaderCode::<Generic>::new(String::new()));
        Self {
            shared_procedure_textures_code: shared_code,
            textures: HashMap::new(),
            names_generator: FunctionNameGenerator::new_shared(),
        }
    }
    
    #[must_use]
    pub fn make_triplanar_mapper(&mut self)-> TriplanarMapper {
        TriplanarMapper::new(self.names_generator.clone())
    }

    #[must_use]
    pub fn animated(&self, uid: ProceduralTextureUid) -> bool {
        if let Some(identified) = self.textures.get(&uid) {
            return identified.texture.animated()
        }
        false
    }

    #[must_use]
    pub fn add(&mut self, target: TextureProcedural3D, name: Option<&str>) -> ProceduralTextureUid {
        let name = self.names_generator.borrow_mut().next_name(name);
        let uid = ProceduralTextureUid(self.textures.len() + 1);
        self.textures.insert(uid, NamedTextureProcedural { texture: target, name });
        uid
    }

    #[must_use]
    pub(crate) fn generate_gpu_code(&self) -> ShaderCode {
        let mut buffer: String = self.shared_procedure_textures_code.to_string();

        if false == buffer.is_empty() {
            buffer.push('\n');
        }
        self.write_gpu_code(&mut buffer).expect("shader code formatting failed");

        ShaderCode::<Generic>::new(buffer)
    }

    fn write_gpu_code(&self, buffer: &mut String) -> anyhow::Result<()> {
        let mut sorted: Vec<(&ProceduralTextureUid, &NamedTextureProcedural)> = self.textures.iter().collect();
        sorted.sort_by_key(|(_, value)| &value.name.0);

        for (_, candidate) in sorted.iter() {
            let utilities = candidate.texture.utilities();
            if false == utilities.is_empty() {
                write!(buffer, "{utilities}")?;
            }
            
            let body = candidate.texture.function_body();
            write_texture_3d_code(body, &candidate.name, buffer)?;
        }
        Self::write_selection_function(&sorted, buffer)?;

        Ok(())
    }

    fn write_selection_function(variants: &Vec<(&ProceduralTextureUid, &NamedTextureProcedural)>, buffer: &mut String) -> anyhow::Result<()> {
        write_texture_3d_selection_function_opening(buffer)?;

        for variant in variants {
            write_texture_3d_selection(&variant.1.name, *variant.0, buffer)?;
        }

        write!(buffer, "return vec3f(0.0);\n}}\n")?;
        Ok(())
    }

    #[must_use]
    pub(crate) fn make_dummy_selection_function() -> ShaderCode {
        let mut result = String::new();
        Self::write_selection_function(&Vec::new(), &mut result).expect("shader code formatting failed");
        ShaderCode::<Generic>::new(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader::code::{FunctionBody, Generic, ShaderCode};
    use more_asserts::assert_gt;
    use crate::shader::conventions;

    #[must_use]
    fn procedural_texture(body: &str) -> TextureProcedural3D {
        TextureProcedural3D::from_simple_body(ShaderCode::<FunctionBody>::new(body.to_string()))
    }

    #[must_use]
    fn make_system_under_test() -> ProceduralTextures {
        ProceduralTextures::new(None)
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
                "{}\nfn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {{\nreturn vec3f(0.0);\n}}\n",
                expected_generated_code
            )
        );
    }

    #[test]
    fn test_new_without_shared_code() {
        let system_under_test = make_system_under_test();

        let generated_code = system_under_test.generate_gpu_code();

        assert_eq!(
            generated_code.to_string(),
            format!("fn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {{\nreturn vec3f(0.0);\n}}\n")
        );
    }

    #[test]
    fn test_add_single_texture() {
        let mut system_under_test = make_system_under_test();
        let texture = procedural_texture("return vec3f(1.0, 0.0, 0.0);");

        let uid = system_under_test.add(texture, Some("test_texture"));
        assert_gt!(uid.0, 0);
        let generated_code = system_under_test.generate_gpu_code();

        assert!(generated_code.as_str().contains(format!("if (texture_index == {})", uid).as_str()));
    }

    #[test]
    fn test_add_multiple_textures() {
        let mut system_under_test = make_system_under_test();

        let first_uid = system_under_test.add(
            procedural_texture("return vec3f(1.0, 0.0, 0.0);"),
            Some("red_texture"),
        );
        let second_uid = system_under_test.add(
            procedural_texture("return vec3f(0.0, 1.0, 0.0);"),
            Some("green_texture"),
        );
        let third_uid = system_under_test.add(
            procedural_texture("return vec3f(0.0, 0.0, 1.0);"),
            Some("blue_texture"),
        );

        assert_ne!(first_uid, second_uid);
        assert_ne!(second_uid, third_uid);
    }

    #[test]
    fn test_generate_gpu_code_multiple_textures() {
        let mut system_under_test = make_system_under_test();
        let first_texture = procedural_texture("return vec3f(1.0, 0.0, 0.0);");
        let second_texture = procedural_texture("return vec3f(0.0, 1.0, 0.0);");
    
        let _ = system_under_test.add(first_texture, Some("red_texture"));
        let _ = system_under_test.add(second_texture, Some("green_texture"));
    
        let actual_code = system_under_test.generate_gpu_code();
        let expected_code = "fn green_texture(point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f)->vec3f{\nreturn vec3f(0.0, 1.0, 0.0);\n}\nfn red_texture(point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f)->vec3f{\nreturn vec3f(1.0, 0.0, 0.0);\n}\nfn procedural_texture_select(texture_index: i32, point: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {\nif (texture_index == 2) { return green_texture(point,normal,time,dp_dx,dp_dy); }\nif (texture_index == 1) { return red_texture(point,normal,time,dp_dx,dp_dy); }\nreturn vec3f(0.0);\n}\n";
        
        assert_eq!(actual_code.to_string(), expected_code);
    }
    
    #[test]
    fn test_add_with_same_function() {
        let mut system_under_test = make_system_under_test();
        let texture = procedural_texture("return vec3f(3.0, 0.0, 0.0);");

        let first = system_under_test.add(texture.clone(), Some("test_texture"));
        let second = system_under_test.add(texture, Some("test_texture"));

        assert_ne!(first, second);
    }
    
    #[test]
    fn test_generate_gpu_code_multiple_calls_same_result() {
        let mut system_under_test = make_system_under_test();
        let texture = procedural_texture("return vec3f(0.8, 0.2, 0.1);");

        let _ = system_under_test.add(texture, Some("orange_texture"));
    
        let first = system_under_test.generate_gpu_code();
        let second = system_under_test.generate_gpu_code();
    
        assert_eq!(first, second);
    }

    #[test]
    fn test_animated_false() {
        let mut system_under_test = make_system_under_test();
        let texture = procedural_texture("return vec3f(0.8, 0.2, 0.1);");
        let uid = system_under_test.add(texture, Some("b"));

        assert_eq!(system_under_test.animated(uid), false);
    }

    #[test]
    fn test_animated_true() {
        let mut system_under_test = make_system_under_test();
        let texture_body = format!("return vec3f({}, 0.2, 0.1);", conventions::PARAMETER_NAME_THE_TIME);
        let texture = procedural_texture(texture_body.as_str());
        let uid = system_under_test.add(texture, Some("a"));

        assert!(system_under_test.animated(uid));
    }
}
