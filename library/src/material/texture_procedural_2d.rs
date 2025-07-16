use crate::shader::code::{FunctionBody, Generic, ShaderCode};

pub struct TextureProcedural2D {
    utilities: ShaderCode<Generic>,
    evaluation: ShaderCode<FunctionBody>,
}

impl TextureProcedural2D {
    #[must_use]
    pub fn new(utilities: ShaderCode<Generic>, evaluation: ShaderCode<FunctionBody>) -> Self {
        Self { utilities, evaluation }
    }

    #[must_use]
    pub(super) fn utilities(&self) -> &ShaderCode<Generic> {
        &self.utilities
    }

    #[must_use]
    pub(super) fn evaluation(&self) -> &ShaderCode<FunctionBody> {
        &self.evaluation
    }
}

