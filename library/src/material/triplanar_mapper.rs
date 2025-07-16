use crate::material::texture_procedural_2d::TextureProcedural2D;
use crate::material::texture_procedural_3d::TextureProcedural3D;
use crate::shader::code::{FunctionBody, Generic, ShaderCode};
use crate::shader::conventions;
use crate::shader::formatting_utils::format_scalar;
use crate::shader::function_name_generator::FunctionNameGenerator;
use more_asserts::assert_gt;
use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;

pub struct TriplanarMapper {
    names_generator: Rc<RefCell<FunctionNameGenerator>>,
}

impl TriplanarMapper {
    #[must_use]
    pub(crate) fn new(names_generator: Rc<RefCell<FunctionNameGenerator>>) -> Self {
        Self { names_generator }
    }
    
    #[must_use]
    pub fn make_triplanar_mapping(&mut self, surface_texture: &TextureProcedural2D, transition_sharpness: f64, name: Option<&str>) -> TextureProcedural3D {
        assert_gt!(transition_sharpness, 0.0, "transition_sharpness must be strictly positive");

        let prefix = name.unwrap_or("texture_2d_procedural");
        let function_name = self.names_generator.borrow_mut().next_name(Some(prefix));

        let evaluation_code = format!("\
            let x: vec3f = {sample_texture}({point_parameter_name}.yz);\n\
            let y: vec3f = {sample_texture}({point_parameter_name}.zx);\n\
            let z: vec3f = {sample_texture}({point_parameter_name}.xy);\n\
            let w = pow( abs({normal_parameter_name}), vec3({transition_sharpness}) );\n\
            return (x*w.x + y*w.y + z*w.z) / (w.x + w.y + w.z);",

            sample_texture = function_name,
            point_parameter_name = conventions::PARAMETER_NAME_THE_POINT,
            normal_parameter_name = conventions::PARAMETER_NAME_THE_NORMAL,
            transition_sharpness = format_scalar(transition_sharpness),
        );

        let mut utilities_code = surface_texture.utilities().to_string();
        write!(
            utilities_code,
            "fn {function_name}({parameter_name}: vec2f)->vec3f{{\n{body}\n}}\n",
            function_name = function_name,
            parameter_name = conventions::PARAMETER_NAME_2D_TEXTURE_COORDINATES,
            body = surface_texture.evaluation(),
        )
        .expect("failed to write utilities code for 2d texture");

        TextureProcedural3D::new(ShaderCode::<Generic>::new(utilities_code), ShaderCode::<FunctionBody>::new(evaluation_code.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::texture_procedural_2d::TextureProcedural2D;
    use crate::shader::code::{FunctionBody, Generic, ShaderCode};

    #[must_use]
    fn make_texture(body: &str) -> TextureProcedural2D {
        let utilities = ShaderCode::<Generic>::new(String::new());
        let evaluation = ShaderCode::<FunctionBody>::new(body.to_string());
        TextureProcedural2D::new(utilities, evaluation)
    }

    #[test]
    fn test_new() {
        let mut system_under_test = TriplanarMapper::new(FunctionNameGenerator::new_shared());

        let texture = make_texture("return vec3f(1.0, 0.0, 0.0);");
        let result = system_under_test.make_triplanar_mapping(&texture, 1.0, None);

        assert_eq!(result.utilities().as_str(), "fn texture_2d_procedural(uv: vec2f)->vec3f{\nreturn vec3f(1.0, 0.0, 0.0);\n}\n");
        assert_eq!(result.function_body().as_str(), "const x: vec3f = texture_2d_procedural(point.yz);\nconst y: vec3f = texture_2d_procedural(point.zx);\nconst z: vec3f = texture_2d_procedural(point.xy);\nconst w = pow( abs(normal), vec3(1.0) );\nreturn (x*w.x + y*w.y + z*w.z) / (w.x + w.y + w.z);");
    }

    #[test]
    fn test_make_triplanar_mapping_with_custom_name() {
        let mut system_under_test = TriplanarMapper::new(FunctionNameGenerator::new_shared());

        let texture = make_texture("return vec3f(1.0, 0.0, 0.0);");
        let result = system_under_test.make_triplanar_mapping(&texture, 1.0, Some("custom_name"));

        assert_eq!(result.utilities().as_str(), "fn custom_name(uv: vec2f)->vec3f{\nreturn vec3f(1.0, 0.0, 0.0);\n}\n");
        assert_eq!(result.function_body().as_str(), "const x: vec3f = custom_name(point.yz);\nconst y: vec3f = custom_name(point.zx);\nconst z: vec3f = custom_name(point.xy);\nconst w = pow( abs(normal), vec3(1.0) );\nreturn (x*w.x + y*w.y + z*w.z) / (w.x + w.y + w.z);");

    }

    #[test]
    fn test_make_triplanar_mapping_different_sharpness_values() {
        let mut system_under_test = TriplanarMapper::new(FunctionNameGenerator::new_shared());

        let texture = make_texture("return vec3f(1.0, 0.0, 0.0);");

        let texture_one = system_under_test.make_triplanar_mapping(&texture, 0.5, None);
        let texture_two = system_under_test.make_triplanar_mapping(&texture, 2.0, None);

        assert!(texture_one.function_body().as_str().contains("vec3(0.5)"));
        assert!(texture_two.function_body().as_str().contains("vec3(2.0)"));
    }

    #[test]
    fn test_make_triplanar_mapping_incorporates_source_texture() {
        let mut system_under_test = TriplanarMapper::new(FunctionNameGenerator::new_shared());

        let utilities_code = "// custom utilities code";
        let texture_code = "return vec3f(0.5, 0.5, 0.5);";

        let texture_2d = TextureProcedural2D::new(
            ShaderCode::<Generic>::new(utilities_code.to_string()),
            ShaderCode::<FunctionBody>::new(texture_code.to_string()),
        );

        let texture_3d = system_under_test.make_triplanar_mapping(&texture_2d, 7.0, None);

        assert!(texture_3d.utilities().as_str().contains(utilities_code));
        assert!(texture_3d.utilities().as_str().contains(texture_code));
    }
}