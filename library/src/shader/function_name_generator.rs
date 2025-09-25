use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use crate::shader::function_name::FunctionName;

pub(crate) struct FunctionNameGenerator {
    synthetic_names_counter: u64,
    used_names: HashSet<FunctionName>,
}

impl FunctionNameGenerator {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            synthetic_names_counter: 0,
            used_names: HashSet::new(),
        }
    }

    #[must_use]
    pub(crate) fn new_shared() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new()))
    }

    #[must_use]
    pub(crate) fn next_name(&mut self, prefix: Option<&str>) -> FunctionName {
        let name_prefix: &str;

        if let Some(prefix) = prefix && false == prefix.trim().is_empty() {
            let result = FunctionName(prefix.to_string());
            if false == self.used_names.contains(&result) {
                self.used_names.insert(result.clone());
                return result;
            }
            name_prefix = prefix;
        } else {
            name_prefix = "generated_function";
        }

        loop {
            self.synthetic_names_counter += 1;
            let result = FunctionName(format!("{}_{}", name_prefix, self.synthetic_names_counter));

            if false == self.used_names.contains(&result) {
                self.used_names.insert(result.clone());
                return result;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let mut system_under_test = FunctionNameGenerator::new();

        let first_name = system_under_test.next_name(None);
        assert_eq!(first_name, FunctionName("generated_function_1".to_string()));
    }

    #[test]
    fn test_next_name_default_prefix() {
        let mut system_under_test = FunctionNameGenerator::new();
        
        let name_one = system_under_test.next_name(None);
        let name_two = system_under_test.next_name(None);
        
        assert_ne!(name_one, name_two);
    }

    #[test]
    fn test_next_name_custom_prefix() {
        let mut system_under_test = FunctionNameGenerator::new();
        
        let name_one = system_under_test.next_name(Some("custom"));
        let name_two = system_under_test.next_name(Some("custom"));
        
        assert_ne!(name_one, name_two);
    }

    #[test]
    fn test_next_name_mixed_prefixes() {
        let mut system_under_test = FunctionNameGenerator::new();
        
        let name_one = system_under_test.next_name(Some("generated_function"));
        let name_two = system_under_test.next_name(None);
        let name_three = system_under_test.next_name(Some("generated_function"));
        
        assert_ne!(name_one, name_two);
        assert_ne!(name_two, name_three);
    }

    #[test]
    fn test_a_lot_of_names_are_unique() {
        const NAMES_TO_GENERATE: usize = 100;

        let mut system_under_test = FunctionNameGenerator::new();
        let mut generated_names = HashSet::new();

        for _ in 0..NAMES_TO_GENERATE {
            let name = system_under_test.next_name(Some("unique"));
            assert!(generated_names.insert(name.clone()), "generated duplicate name: {:?}", name);
        }
    }

    #[test]
    fn test_empty_prefix() {
        let mut system_under_test = FunctionNameGenerator::new();
        
        let name = system_under_test.next_name(Some(""));
        assert_eq!(name, FunctionName("generated_function_1".to_string()));
    }

    #[test]
    fn test_alternating_prefixes() {
        let mut system_under_test = FunctionNameGenerator::new();
        
        let name_one = system_under_test.next_name(Some("a"));
        let name_two = system_under_test.next_name(Some("b"));
        let name_three = system_under_test.next_name(Some("a"));
        let name_four = system_under_test.next_name(Some("b"));
        
        assert_ne!(name_one, name_two);
        assert_ne!(name_two, name_three);
        assert_ne!(name_three, name_four);
    }
}