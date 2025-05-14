use crate::sdf::dfs;
use crate::sdf::function_body_dossier::FunctionBodyDossier;
use crate::sdf::named_sdf::{NamedSdf, UniqueName};
use crate::sdf::shader_code::{format_sdf_declaration, format_sdf_invocation, FunctionBody, ShaderCode};
use crate::sdf::shader_code_dossier::ShaderCodeDossier;
use crate::sdf::shader_function_name::FunctionName;
use crate::sdf::stack::Stack;
use crate::utils::uid_generator::UidGenerator;
use std::collections::HashMap;

pub struct SdfRegistrator {
    sdf_bodies: FunctionBodyDossier,
    uid_generator: UidGenerator,
    registered: HashMap<UniqueName, NamedSdf>,
}

pub(crate) struct SdfCodeGenerator {
    sdf_bodies: FunctionBodyDossier,
    registered: HashMap<UniqueName, NamedSdf>,
}

impl SdfCodeGenerator {
    #[must_use]
    pub(crate) fn new(collection: SdfRegistrator) -> Self {
        Self {
            sdf_bodies: collection.sdf_bodies,
            registered: collection.registered,
        }
    }

    #[must_use]
    pub(crate) fn registrations(&self) -> &HashMap<UniqueName, NamedSdf> {
        &self.registered
    }
    
    pub(crate) fn generate_shared_code(self, buffer: &mut String) {
        self.sdf_bodies.format_occurred_multiple_times(buffer);
    }
    
    pub(crate) fn generate_unique_code_for(&self, target: &NamedSdf, buffer: &mut String) -> FunctionName {
        assert!(self.registered.contains_key(target.name()));
        
        struct Context<'a> {
            descendant_bodies: Stack<ShaderCode<FunctionBody>>,
            descendant_bodies_deduplicated: Stack<ShaderCode<FunctionBody>>,
            last_body_name: Option<&'a FunctionName>,
        }

        let mut context = Context {
            descendant_bodies: Stack::new(),
            descendant_bodies_deduplicated: Stack::new(),
            last_body_name: None,
        };

        dfs::depth_first_search(target.sdf(), &mut context, |candidate, context, levels_below| {
            let body = candidate.produce_body(&mut context.descendant_bodies, None);
            
            let occurrences = self.sdf_bodies.try_find(&body);
            if let Some(occurrences) = occurrences.filter(|o| o.occurrences() > 1) {
                for _ in candidate.children() {
                    context.descendant_bodies_deduplicated.pop();
                }
                context.descendant_bodies_deduplicated.push(format_sdf_invocation(occurrences.name()));
                context.last_body_name = Some(occurrences.name());
            } else {
                let body = candidate.produce_body(&mut context.descendant_bodies_deduplicated, Some(levels_below));
                context.descendant_bodies_deduplicated.push(body);
                context.last_body_name = None;
            }

            context.descendant_bodies.push(body);
        });
        
        debug_assert_eq!(context.descendant_bodies.size(), 1);
        debug_assert_eq!(context.descendant_bodies_deduplicated.size(), 1);
        
        if let Some(last_body_name) = context.last_body_name {
            last_body_name.clone()
        } else {
            let sdf_name = FunctionName::from(target.name());
            format_sdf_declaration(&context.descendant_bodies_deduplicated.pop(), &sdf_name, buffer);
            sdf_name   
        }
    }
}

impl SdfRegistrator {
    pub fn add(&mut self, target: &NamedSdf) {
        let unique = self.registered.insert(target.name().clone(), target.clone());
        assert!(unique.is_none(), "name {} of given sdf is not unique", target.name());
        
        let mut context = (self, Stack::<ShaderCode<FunctionBody>>::new(), target.name().as_str());
        
        dfs::depth_first_search(target.sdf(), &mut context, |candidate, context, levels_below| {
            let (this, descendant_bodies, target_name) = context;
            
            let body = candidate.produce_body(descendant_bodies, None);
            let body_seen_first_time = false == this.sdf_bodies.try_account_occurrence(&body, candidate.clone());
            
            if body_seen_first_time {
                let function = FunctionName(format!("sdf_{}_{}", target_name, this.uid_generator.next().0));
                let dossier = ShaderCodeDossier::new(function, candidate.clone(), levels_below);
                this.sdf_bodies.register(body.clone(), dossier);
            }

            descendant_bodies.push(body);
        });

        let (_, children, _) = context;
        debug_assert_eq!(children.size(), 1);
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            sdf_bodies: FunctionBodyDossier::new(),
            uid_generator: UidGenerator::new(),
            registered: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::sdf::named_sdf::UniqueName;
    use crate::sdf::sdf::Sdf;
    use crate::sdf::sdf_box::SdfBox;
    use crate::sdf::sdf_sphere::SdfSphere;
    use crate::sdf::sdf_union::SdfUnion;
    use std::rc::Rc;

    #[test]
    #[should_panic]
    fn test_identical_names_registration_attempt() {
        let name = UniqueName::new("the_name".to_string());
        let named = NamedSdf::new(make_dummy_sdf(), name.clone());

        let mut registrator_under_test = SdfRegistrator::new();
        registrator_under_test.add(&named);
        registrator_under_test.add(&named);
    }
    
    #[test]
    fn test_single_one_node_sdf() {
        let geometry = make_dummy_sdf();
        let name = UniqueName::new("the_name".to_string());
        let named = NamedSdf::new(geometry.clone(), name.clone());
        
        let mut registrator_under_test = SdfRegistrator::new();
        registrator_under_test.add(&named);
        
        let generator_under_test = SdfCodeGenerator::new(registrator_under_test);
        let mut actual_code: String = String::new();
        let actual_name = generator_under_test.generate_unique_code_for(&named, &mut actual_code);
        
        let expected_name = FunctionName::from(&name);
        let expected_code = make_single_function_declaration(geometry.clone(), &expected_name);
        
        assert_no_shared_code(generator_under_test);
        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_code, expected_code);
    }

    #[test]
    fn test_two_same_one_node_sdf() {
        let geometry = make_dummy_sdf();
        
        let first_name = UniqueName::new("the_first".to_string());
        let first_named = NamedSdf::new(geometry.clone(), first_name.clone());
        
        let second_name = UniqueName::new("the_second".to_string());
        let second_named = NamedSdf::new(geometry.clone(), second_name.clone());

        let mut registrator_under_test = SdfRegistrator::new();
        registrator_under_test.add(&first_named);
        registrator_under_test.add(&second_named);

        let generator_under_test = SdfCodeGenerator::new(registrator_under_test);
        
        let mut actual_code: String = String::new();
        let actual_first_name = generator_under_test.generate_unique_code_for(&first_named, &mut actual_code);
        let actual_second_name = generator_under_test.generate_unique_code_for(&first_named, &mut actual_code);
        assert!(actual_code.is_empty());
        assert_eq!(actual_first_name, actual_second_name);

        generator_under_test.generate_shared_code(&mut actual_code);
        let expected_code = make_single_function_declaration(geometry.clone(), &actual_first_name);
        assert_eq!(actual_code, expected_code);
    }
    
    #[test]
    fn test_single_tree_with_unique_sdf() {
        let tree = SdfUnion::new(
            SdfUnion::new(
                SdfUnion::new(
                    SdfBox::new(Vector::new(1.0, 2.0, 3.0)),
                    SdfBox::new(Vector::new(5.0, 7.0, 11.0)),
                ),
                SdfSphere::new_offset(13.0, Point::new(-17.0, -19.0, -23.0)),
            ),
            SdfSphere::new_offset(29.0, Point::new(31.0, 37.0, 41.0)),
        );

        let name = UniqueName::new("the_name".to_string());

        let (actual_name, actual_code, generator_under_test) = generate_code(tree.clone(), name.clone());

        let expected_name = FunctionName::from(&name);
        let expected_code = "fn sdf_the_name(point: vec3f) -> f32 { \
        var left_3: f32;\n { \
        var left_2: f32;\n { \
        var left_1: f32;\n { \
        let q = abs(point)-vec3f(1.0,2.0,3.0); \
        left_1 = length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0); } \
        var right_1: f32;\n { \
        let q = abs(point)-vec3f(5.0,7.0,11.0); \
        right_1 = length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0); } \
        left_2 = min(left_1,right_1); } \
        var right_2: f32;\n { \
        right_2 = length((point-vec3f(-17.0,-19.0,-23.0)))-13.0; } \
        left_3 = min(left_2,right_2); } \
        var right_3: f32;\n { \
        right_3 = length((point-vec3f(31.0,37.0,41.0)))-29.0; } \
        return min(left_3,right_3); }\n";

        assert_no_shared_code(generator_under_test);
        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_code, expected_code);
    }
    
    #[test]
    fn test_tree_with_one_level_duplications() {
        let tree = SdfUnion::new(
            SdfSphere::new(17.0),
            SdfSphere::new(17.0),
        );

        let name = UniqueName::new("test".to_string());
        
        let (actual_name, actual_code, generator_under_test) = generate_code(tree.clone(), name.clone());

        let mut actual_shared_code = String::new();
        generator_under_test.generate_shared_code(&mut actual_shared_code);
        
        let expected_name = FunctionName::from(&name);
        let expected_code = "fn sdf_test(point: vec3f) -> f32 { var left_1: f32;\n { left_1 = sdf_test_1(point); } var right_1: f32;\n { right_1 = sdf_test_1(point); } return min(left_1,right_1); }\n";
        let expected_shared_code = "fn sdf_test_1(point: vec3f) -> f32 { return length(point)-17.0; }\n";
        
        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_shared_code, expected_shared_code);
        assert_eq!(actual_code, expected_code);
    }

    #[test]
    fn test_tree_with_multiple_levels_of_duplications() {
        let tree = SdfUnion::new(
            SdfSphere::new(17.0),
            SdfUnion::new(
                SdfUnion::new(
                    SdfSphere::new(17.0),
                    SdfSphere::new(17.0),
                ),
                SdfUnion::new(
                    SdfSphere::new(17.0),
                    SdfSphere::new(17.0),
                ),
            ),
        );

        let name = UniqueName::new("test".to_string());

        let (actual_name, actual_code, generator_under_test) = generate_code(tree.clone(), name.clone());

        let mut actual_shared_code = String::new();
        generator_under_test.generate_shared_code(&mut actual_shared_code);

        let expected_name = FunctionName::from(&name);
        let expected_code = "fn sdf_test(point: vec3f) -> f32 { var left_3: f32;\n { left_3 = sdf_test_1(point); } var right_3: f32;\n { var left_2: f32;\n { left_2 = sdf_test_2(point); } var right_2: f32;\n { right_2 = sdf_test_2(point); } right_3 = min(left_2,right_2); } return min(left_3,right_3); }\n";
        let expected_shared_code = "fn sdf_test_1(point: vec3f) -> f32 { return length(point)-17.0; }\nfn sdf_test_2(point: vec3f) -> f32 { var left: f32;\n { left = sdf_test_1(point); } var right: f32;\n { right = sdf_test_1(point); } return min(left,right); }\n";

        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_shared_code, expected_shared_code);
        assert_eq!(actual_code, expected_code);
    }
    
    #[must_use]
    fn generate_code(sdf: Rc<dyn Sdf>, name: UniqueName) -> (FunctionName, String, SdfCodeGenerator) {
        let named = NamedSdf::new(sdf, name);
        let mut registrator_under_test = SdfRegistrator::new();
        
        registrator_under_test.add(&named);
        let generator_under_test = SdfCodeGenerator::new(registrator_under_test);
       
        let mut actual_code: String = String::new();
        let actual_name = generator_under_test.generate_unique_code_for(&named, &mut actual_code);

        (actual_name, actual_code, generator_under_test)
    }

    #[must_use]
    fn make_single_function_declaration(geometry: Rc<dyn Sdf>, expected_name: &FunctionName) -> String {
        let mut result: String = String::new();
        format_sdf_declaration(&geometry.produce_body(&mut Stack::new(), None), &expected_name, &mut result);
        result
    }

    #[must_use]
    fn make_dummy_sdf() -> Rc<dyn Sdf> {
        SdfSphere::new(17.0)
    }

    fn assert_no_shared_code(system_under_test: SdfCodeGenerator) {
        let mut buffer = String::new();
        system_under_test.generate_shared_code(&mut buffer);
        assert!(buffer.is_empty());
    }
}
