use more_asserts::assert_gt;
use crate::material::texture_procedural::TextureProcedural;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::conventions;
use crate::shader::formatting_utils::format_scalar;

#[must_use]
pub fn make_checkerboard_texture(scale: f64) -> TextureProcedural {
    assert_gt!(scale, 0.0);
    let code = format!("\
        var result = vec3f(1.0);\n\
        if ( i32( dot(floor({point_parameter_name} * {scale}), vec3f(1.0)) ) % 2 == 0 ) {{\n\
            result = vec3f(0.0);\n\
        }}\n\
        return result;\n",
        scale = format_scalar(scale),
        point_parameter_name = conventions::PARAMETER_NAME_THE_POINT,
    );
    
    TextureProcedural::new(ShaderCode::<FunctionBody>::new(code.to_string()))
}