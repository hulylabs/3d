﻿use std::rc::Rc;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_function_name::FunctionName;

pub(crate) struct ShaderCodeDossier {
    name: FunctionName,
    sources: Vec<Rc<dyn Sdf>>,
    children_levels_below: usize,
}

impl ShaderCodeDossier {
    #[must_use]
    pub(crate) fn new(host: FunctionName, source: Rc<dyn Sdf>, children_levels_below: usize) -> Self {
        Self { name: host, sources: vec![source], children_levels_below }
    }

    #[must_use]
    pub(crate) const fn name(&self) -> &FunctionName {
        &self.name
    }

    pub(crate) fn write_another_usage(&mut self, source: Rc<dyn Sdf>) {
        self.sources.push(source);
    }

    #[must_use]
    pub(crate) fn occurrences(&self) -> usize {
        self.sources.len()
    }

    #[must_use]
    pub(crate) fn children_levels_below(&self) -> usize {
        self.children_levels_below
    }

    #[must_use]
    pub(crate) fn any_source(&self) -> Rc<dyn Sdf> {
        self.sources[0].clone()
    }

    #[must_use]
    pub(crate) fn sources(&self) -> &Vec<Rc<dyn Sdf>> {
        &self.sources
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::dummy_sdf::tests::make_dummy_sdf;
    use crate::sdf::framework::shader_function_name::FunctionName;

    #[test]
    fn test_disjoint_set() {
        // TODO: implement me
    }

    #[test]
    fn test_construction() {
        let expected_name = FunctionName("name".to_string());
        let expected_levels_below = 17;
        let system_under_test = ShaderCodeDossier::new(expected_name.clone(), make_dummy_sdf(), expected_levels_below);
        
        assert_eq!(system_under_test.occurrences(), 1);
        assert_eq!(system_under_test.name(), &expected_name);
        assert_eq!(system_under_test.children_levels_below(), expected_levels_below);
    }
    
    #[test]
    fn test_write_another_usage() {
        let expected_levels_below = 17;
        let mut system_under_test = ShaderCodeDossier::new(FunctionName("name".to_string()), make_dummy_sdf(), expected_levels_below);
        
        system_under_test.write_another_usage(make_dummy_sdf());
        assert_eq!(system_under_test.occurrences(), 2);
        assert_eq!(system_under_test.children_levels_below(), expected_levels_below);
        
        system_under_test.write_another_usage(make_dummy_sdf());
        assert_eq!(system_under_test.occurrences(), 3);
        assert_eq!(system_under_test.children_levels_below(), expected_levels_below);
    }
}