use crate::shader::code::{FunctionBody, Generic, ShaderCode};

#[derive(Clone)]
pub struct TextureProcedural3D {
    utilities: ShaderCode<Generic>,
    function_body: ShaderCode<FunctionBody>,
}

impl TextureProcedural3D {
    #[must_use]
    pub fn new(utilities: ShaderCode<Generic>, function_body: ShaderCode<FunctionBody>) -> Self {
        Self { utilities, function_body }
    }
    
    #[must_use]
    pub fn from_simple_body(function_body: ShaderCode<FunctionBody>) -> Self {
        Self { utilities: ShaderCode::<Generic>::new(String::new()), function_body }
    }

    #[must_use]
    pub(crate) fn function_body(&self) -> &ShaderCode<FunctionBody> {
        &self.function_body
    }

    #[must_use]
    pub(crate) fn utilities(&self) -> &ShaderCode<Generic> {
        &self.utilities
    }
}
