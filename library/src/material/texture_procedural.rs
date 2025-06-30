use crate::shader::code::{FunctionBody, ShaderCode};

#[derive(Clone)]
pub struct TextureProcedural {
    function_body: ShaderCode<FunctionBody>,
}

impl TextureProcedural {
    #[must_use]
    pub fn new(function_body: ShaderCode<FunctionBody>) -> Self {
        Self { function_body }
    }

    #[must_use]
    pub(crate) fn function_body(&self) -> &ShaderCode<FunctionBody> {
        &self.function_body
    }
}
