use crate::scene::sdf::equality_sets::EqualitySets;
use crate::scene::sdf::shader_code_dossier::ShaderCodeDossier;
use crate::scene::sdf::sdf::Sdf;
use crate::scene::sdf::shader_code::{format_sdf_declaration, format_sdf_invocation, FunctionBody, ShaderCode};
use crate::scene::sdf::shader_function_name::FunctionName;
use crate::scene::sdf::stack::Stack;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub(super) struct FunctionBodyDossier {
    dossier_of_body: HashMap<ShaderCode<FunctionBody>, ShaderCodeDossier>,
    used_names: HashSet<FunctionName>,
}

impl FunctionBodyDossier {
    #[must_use]
    pub(super) fn new() -> Self {
        Self { dossier_of_body: HashMap::new(), used_names: HashSet::new(), }
    }
    
    #[must_use]
    pub(super) fn try_find(&self, shader_code: &ShaderCode<FunctionBody>) -> Option<&ShaderCodeDossier> {
        self.dossier_of_body.get(shader_code)
    }
    
    #[must_use]
    pub(super) fn try_increment_occurrence(&mut self, shader_code: &ShaderCode<FunctionBody>, instance: Rc<dyn Sdf>) -> bool {
        let dossier = self.dossier_of_body.get_mut(shader_code);
        if let Some(dossier) = dossier {
            dossier.increment(instance);
            true
        } else {
            false
        }
    }
    
    pub(super) fn register(&mut self, shader_code: ShaderCode<FunctionBody>, shader_dossier: ShaderCodeDossier) {
        assert!(self.used_names.insert(shader_dossier.name().clone()), "non-unique function name");

        let previous = self.dossier_of_body.insert(shader_code, shader_dossier);
        assert!(previous.is_none(), "duplicate code bod occurrence");
    }
    
    #[must_use]
    fn sort_multiple_occurrences_bottom_up(&self) -> Vec<&ShaderCodeDossier> {
        let mut result: Vec<&ShaderCodeDossier> = Vec::with_capacity(self.dossier_of_body.len());
        for (_, dossier) in self.dossier_of_body.iter() {
            if dossier.occurrences() > 1 {
                result.push(dossier);
            }
        }
        result.sort_by_key(|x| x.children_levels_below());
        result
    }
    
    pub(super) fn format_occurred_multiple_times(&self, buffer: &mut String) {
        let bottom_up_bodies = self.sort_multiple_occurrences_bottom_up();
        let equality = EqualitySets::new(&bottom_up_bodies);
        
        let mut formatted: HashMap<*const dyn Sdf, ShaderCode<FunctionBody>> = HashMap::with_capacity(bottom_up_bodies.len());
        let mut children = Stack::<ShaderCode<FunctionBody>>::new();
        
        for dossier in bottom_up_bodies {
            for child in dossier.source().children() {
                let reference = equality.get_equality_root(child);
                let successor_body = formatted.get(&Rc::as_ptr(&reference)).unwrap();
                children.push(successor_body.clone());
            }
            
            let current_body = dossier.source().produce_body(&mut children);
            debug_assert_eq!(children.size(), 0);
            format_sdf_declaration(&current_body, dossier.name(), buffer);
            buffer.push('\n');
            
            let reference = equality.get_equality_root(dossier.source());
            formatted.insert(reference.as_ref(), format_sdf_invocation(dossier.name()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::sdf::dummy_sdf::tests::DummySdf;
    use crate::scene::sdf::shader_code::conventions;

    #[test]
    fn test_construction() {
        let system_under_test = FunctionBodyDossier::new();
        let dummy_code = ShaderCode::<FunctionBody>::new("return any".to_string());
        
        let found = system_under_test.try_find(&dummy_code);
        assert!(found.is_none());

        assert_multiple_occurrence_format_is_empty(system_under_test);
    }

    #[test]
    fn test_try_increment_occurrence_once() {
        let mut system_under_test = FunctionBodyDossier::new();
        let code_one = ShaderCode::<FunctionBody>::new("return 1.0".to_string());
        let code_seven = ShaderCode::<FunctionBody>::new("return 7.0".to_string());

        fn test_single_occurrence(system_under_test: &mut FunctionBodyDossier, function: String, code: ShaderCode<FunctionBody>) {
            let source = Rc::new(DummySdf::default());
            let found = system_under_test.try_increment_occurrence(&code, source.clone());
            assert_eq!(false, found);
            system_under_test.register(code.clone(), ShaderCodeDossier::new(FunctionName(function), source, 0));
        }

        test_single_occurrence(&mut system_under_test, "dummy_a".to_string(), code_one);
        test_single_occurrence(&mut system_under_test, "dummy_b".to_string(), code_seven);

        assert_multiple_occurrence_format_is_empty(system_under_test);
    }

    #[test]
    fn test_try_increment_occurrence_twice() {
        let mut system_under_test = FunctionBodyDossier::new();
        const SINGLE_RETURN_VALUE: &str = "1.0";
        let code_one = ShaderCode::<FunctionBody>::new(format!("return {}", SINGLE_RETURN_VALUE));
        const MULTIPLE_RETURN_VALUE: &str = "7.0";
        let code_seven = ShaderCode::<FunctionBody>::new(format!("return {}", MULTIPLE_RETURN_VALUE));
        
        system_under_test.register(code_one.clone(), ShaderCodeDossier::new(FunctionName("single".to_string()), Rc::new(DummySdf::new(SINGLE_RETURN_VALUE)), 0));

        let multiple_occurrences_function = "multiple";
        fn used_multiple_times() -> Rc<dyn Sdf> { Rc::new(DummySdf::new(MULTIPLE_RETURN_VALUE)) }
        system_under_test.register(code_seven.clone(), ShaderCodeDossier::new(FunctionName(multiple_occurrences_function.to_string()), used_multiple_times(), 0));
        let _ = system_under_test.try_increment_occurrence(&code_seven, used_multiple_times());
        
        let mut buffer: String = String::new();
        system_under_test.format_occurred_multiple_times(&mut buffer);
        let expected_buffer = format!("fn {}({}: vec3f) -> f32 {{ {}; }}\n", multiple_occurrences_function, conventions::THE_POINT_PARAMETER_NAME, code_seven);
        assert_eq!(buffer, expected_buffer);
    }

    fn assert_multiple_occurrence_format_is_empty(system_under_test: FunctionBodyDossier) {
        let mut buffer: String = String::new();
        system_under_test.format_occurred_multiple_times(&mut buffer);
        assert!(buffer.is_empty(), "'format_occurred_multiple_times' expected to produce nothing, but '{}' was found ", buffer);
    }
}