use derive_more::Display;
use crate::sdf::named_sdf::UniqueSdfClassName;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Display, Ord, PartialOrd)]
#[display("{}", _0)]
pub(crate) struct FunctionName(pub String);

impl From<&UniqueSdfClassName> for FunctionName {
    #[must_use]
    fn from(value: &UniqueSdfClassName) -> Self {
        FunctionName(format!("sdf_{}", value))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;

    #[test]
    fn test_display() {
        let expected_display = "sdf_example";
        let system_under_test = FunctionName(expected_display.to_string());
        
        let actual_display = format!("{}", system_under_test);
        
        assert_eq!(actual_display, expected_display);
    }

    #[test]
    fn test_from_unique_name() {
        let name = "my_shape";
        let system_under_test = FunctionName::from(&UniqueSdfClassName::new(name.to_string()));
        assert_eq!(system_under_test.0, "sdf_my_shape");
    }

    #[test]
    fn test_hash_equality() {
        let equal_one = FunctionName("sdf_a".to_string());
        let equal_two = FunctionName("sdf_a".to_string());
        let different = FunctionName("sdf_b".to_string());

        let mut set = HashSet::new();
        set.insert(equal_one.clone());

        assert!(set.contains(&equal_two));
        assert!(!set.contains(&different));
    }
}