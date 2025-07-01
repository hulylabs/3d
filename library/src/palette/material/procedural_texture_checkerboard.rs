use crate::material::texture_procedural::TextureProcedural;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use crate::shader::formatting_utils::format_scalar;

#[must_use]
pub fn make_checkerboard_texture(scale: f64) -> TextureProcedural {
    let code = format!("\
        let parameter = floor({point_parameter_name} * {scale});\
        var result = vec3f(1.0);
        if (i32(dot(parameter, vec3f(1.0))) % 2 == 0) {{\n\
            result = vec3f(0.0);\n\
        }}\n\
        return result;\n",
        scale = format_scalar(scale),
        point_parameter_name = conventions::PARAMETER_NAME_THE_POINT,
    );
    
    TextureProcedural::new(ShaderCode::<FunctionBody>::new(code.to_string()))
}