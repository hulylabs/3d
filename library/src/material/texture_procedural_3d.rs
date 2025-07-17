use crate::shader::code::{FunctionBody, Generic, ShaderCode};
use crate::shader::conventions;

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

    #[must_use]
    pub(crate) fn animated(&self) -> bool {
        self.function_body.as_str().contains(conventions::PARAMETER_NAME_THE_TIME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_animated_true() {
        let function_body = format!("return vec3(sin({time_parameter}), cos({time_parameter}), 0.0);", time_parameter=conventions::PARAMETER_NAME_THE_TIME);
        let function_body = ShaderCode::<FunctionBody>::new(function_body);
        let system_under_test = TextureProcedural3D::from_simple_body(function_body);
        
        assert!(system_under_test.animated());
    }
    
    #[test]
    fn test_animated_false() {
        let function_body = ShaderCode::<FunctionBody>::new("return vec3(1.0, 0.0, 0.0);".to_string());
        let utilities = format!("fn time({time_parameter}: f32) -> f32 {{ return {time_parameter}; }}", time_parameter=conventions::PARAMETER_NAME_THE_TIME);
        let utilities = ShaderCode::<Generic>::new(utilities);
        let system_under_test = TextureProcedural3D::new(utilities, function_body);
        
        assert!(!system_under_test.animated());
    }
}