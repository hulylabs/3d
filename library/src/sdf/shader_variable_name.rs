use derive_more::Display;

#[derive(Display)]
pub(super) struct ShaderVariableName(String);

impl ShaderVariableName {
    #[must_use]
    pub(super) fn new(name: &str, level: Option<usize>) -> ShaderVariableName {
        if let Some(level) = level { 
            Self(format!("{}_{}", name, level))
        } else { 
            Self(name.to_string())
        }
    }
}

impl From<&ShaderVariableName> for String {
    fn from(name: &ShaderVariableName) -> Self {
        name.0.clone()
    }
}

impl From<ShaderVariableName> for String {
    fn from(name: ShaderVariableName) -> Self {
        name.0
    }
}

impl AsRef<str> for ShaderVariableName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let system_under_test = ShaderVariableName::new("system_under_test", Some(1));
        let display = format!("{}", system_under_test);
        assert_eq!(display, "system_under_test_1");
    }
}