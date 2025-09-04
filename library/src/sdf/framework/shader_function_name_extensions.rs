use crate::sdf::framework::named_sdf::UniqueSdfClassName;
use crate::shader::function_name::FunctionName;

impl From<&UniqueSdfClassName> for FunctionName {
    fn from(value: &UniqueSdfClassName) -> Self {
        FunctionName(format!("sdf_{value}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::named_sdf::UniqueSdfClassName;

    #[test]
    fn test_from_unique_name() {
        let name = "my_shape";
        let system_under_test = FunctionName::from(&UniqueSdfClassName::new(name.to_string()));
        assert_eq!(system_under_test.0, "sdf_my_shape");
    }
}