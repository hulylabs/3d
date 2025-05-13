use std::rc::Rc;
use std::fmt::{Display, Formatter};
use crate::scene::sdf::sdf::Sdf;

pub(crate) struct NamedSdf {
    sdf: Rc<dyn Sdf>, 
    name: UniqueName,
}

impl NamedSdf {
    #[must_use]
    pub(crate) const fn new(sdf: Rc<dyn Sdf>, name: UniqueName) -> Self {
        Self { sdf, name }
    }

    #[must_use]
    pub(crate) fn sdf(&self) -> Rc<dyn Sdf> {
        self.sdf.clone()
    }

    #[must_use]
    pub(crate) const fn name(&self) -> &UniqueName {
        &self.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct UniqueName(pub String);

impl Display for UniqueName {
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
        let system_under_test = UniqueName(expected_display.to_string());

        let actual_display = format!("{}", system_under_test);

        assert_eq!(actual_display, expected_display);
    }

    #[test]
    fn test_equality_and_hash() {
        use std::collections::HashSet;

        let equal_name_one = UniqueName("abc".to_string());
        let equal_name_two = UniqueName("abc".to_string());
        let different_name = UniqueName("xyz".to_string());

        assert_eq!(equal_name_one, equal_name_two);
        assert_ne!(equal_name_one, different_name);

        let mut set = HashSet::new();
        set.insert(equal_name_one.clone());
        assert!(set.contains(&equal_name_two));
        assert!(!set.contains(&different_name));
    }
}