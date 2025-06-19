use std::rc::Rc;
use std::fmt::{Display, Formatter};
use crate::sdf::framework::sdf_base::Sdf;

#[derive(Clone)]
pub struct NamedSdf {
    sdf: Rc<dyn Sdf>, 
    name: UniqueSdfClassName,
}

impl NamedSdf {
    #[must_use]
    pub const fn new(sdf: Rc<dyn Sdf>, name: UniqueSdfClassName) -> Self {
        Self { sdf, name }
    }
    
    #[must_use]
    pub const fn name(&self) -> &UniqueSdfClassName {
        &self.name
    }
    
    #[must_use]
    pub(crate) fn sdf(&self) -> Rc<dyn Sdf> {
        self.sdf.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct UniqueSdfClassName(String);

impl UniqueSdfClassName {
    #[must_use]
    pub fn new(name: String) -> Self {
        if name.chars().all(|c| c.is_ascii_alphabetic() || c == '_') {
            UniqueSdfClassName(name)
        } else {
            panic!("'{}' is invalid: names must contain only letters and underscores", name)
        }
    }

    #[must_use]
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for UniqueSdfClassName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let expected_display = "display_name";
        let system_under_test = UniqueSdfClassName(expected_display.to_string());

        let actual_display = format!("{}", system_under_test);

        assert_eq!(actual_display, expected_display);
    }

    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashSet;

        let equal_name_one = UniqueSdfClassName("abc".to_string());
        let equal_name_two = UniqueSdfClassName("abc".to_string());
        let different_name = UniqueSdfClassName("xyz".to_string());

        assert_eq!(equal_name_one, equal_name_two);
        assert_ne!(equal_name_one, different_name);

        let mut set = HashSet::new();
        set.insert(equal_name_one.clone());
        assert!(set.contains(&equal_name_two));
        assert!(!set.contains(&different_name));
    }
}