use derive_more::Display;

#[derive(Display)]
pub struct VariableName(String);

impl VariableName {
    #[must_use]
    pub(crate) fn new(name: &str, level: Option<usize>) -> VariableName {
        if let Some(level) = level { 
            Self(format!("{name}_{level}"))
        } else { 
            Self(name.to_string())
        }
    }
}

impl From<&VariableName> for String {
    fn from(name: &VariableName) -> Self {
        name.0.clone()
    }
}

impl From<VariableName> for String {
    fn from(name: VariableName) -> Self {
        name.0
    }
}

impl AsRef<str> for VariableName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let system_under_test = VariableName::new("system_under_test", Some(1));
        let display = format!("{}", system_under_test);
        assert_eq!(display, "system_under_test_1");
    }
}