use crate::sdf::framework::dfs;
use crate::sdf::framework::function_body_dossier::FunctionBodyDossier;
use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
use crate::sdf::framework::sdf_registrator::SdfRegistrator;
use crate::sdf::framework::sdf_shader_code::{format_sdf_declaration, format_sdf_invocation};
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::function_name::FunctionName;
use std::collections::HashMap;

pub(crate) struct SdfCodeGenerator {
    sdf_bodies: FunctionBodyDossier,
    registered: HashMap<UniqueSdfClassName, NamedSdf>,
}

impl SdfCodeGenerator {
    #[must_use]
    pub(crate) fn new(collection: SdfRegistrator) -> Self {
        let (sdf_bodies, registered) = collection.registrations();
        Self {
            sdf_bodies,
            registered,
        }
    }

    #[must_use]
    pub(crate) fn registrations(&self) -> &HashMap<UniqueSdfClassName, NamedSdf> {
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
                for _ in candidate.descendants() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::Vector;
    use crate::sdf::composition::sdf_union::SdfUnion;
    use crate::sdf::object::sdf_box::SdfBox;
    use crate::sdf::object::sdf_sphere::SdfSphere;
    use crate::sdf::transformation::sdf_translation::SdfTranslation;
    use std::rc::Rc;
    use crate::sdf::framework::sdf_base::Sdf;

    #[test]
    fn test_single_one_node_sdf() {
        let geometry = make_dummy_sdf();
        let name = UniqueSdfClassName::new("the_name".to_string());
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

        let first_name = UniqueSdfClassName::new("the_first".to_string());
        let first_named = NamedSdf::new(geometry.clone(), first_name.clone());

        let second_name = UniqueSdfClassName::new("the_second".to_string());
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
                SdfTranslation::new(Vector::new(-17.0, -19.0, -23.0), SdfSphere::new(13.0)),
            ),
            SdfTranslation::new(Vector::new(31.0, 37.0, 41.0), SdfSphere::new(29.0)),
        );

        let name = UniqueSdfClassName::new("the_name".to_string());

        let (actual_name, actual_code, generator_under_test) = generate_code(tree.clone(), name.clone());

        let expected_name = FunctionName::from(&name);
        let expected_code = "fn sdf_the_name(point: vec3f, time: f32) -> f32 {\nvar left_3: f32;\n{\nvar left_2: f32;\n{\nvar left_1: f32;\n{\nlet q = abs(point)-vec3f(1.0,2.0,3.0);\nleft_1 = length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);\n}\nvar right_1: f32;\n{\nlet q = abs(point)-vec3f(5.0,7.0,11.0);\nright_1 = length(max(q,vec3f(0.0))) + min(max(q.x,max(q.y,q.z)),0.0);\n}\n\nleft_2 = min(left_1,right_1);\n}\nvar right_2: f32;\n{\nvar operand_1: f32;\n{\nlet point = point-vec3f(-17.0,-19.0,-23.0);\n{\noperand_1 = length(point)-13.0;\n}\n}\nright_2 = operand_1;\n}\n\nleft_3 = min(left_2,right_2);\n}\nvar right_3: f32;\n{\nvar operand_1: f32;\n{\nlet point = point-vec3f(31.0,37.0,41.0);\n{\noperand_1 = length(point)-29.0;\n}\n}\nright_3 = operand_1;\n}\n\nreturn min(left_3,right_3);\n}\n";

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

        let name = UniqueSdfClassName::new("test".to_string());

        let (actual_name, actual_code, generator_under_test) = generate_code(tree.clone(), name.clone());

        let mut actual_shared_code = String::new();
        generator_under_test.generate_shared_code(&mut actual_shared_code);

        let expected_name = FunctionName::from(&name);
        let expected_code = "fn sdf_test(point: vec3f, time: f32) -> f32 {\nvar left_1: f32;\n{\nleft_1 = sdf_test_1(point,time);\n}\nvar right_1: f32;\n{\nright_1 = sdf_test_1(point,time);\n}\n\nreturn min(left_1,right_1);\n}\n";
        let expected_shared_code = "fn sdf_test_1(point: vec3f, time: f32) -> f32 {\nreturn length(point)-17.0;\n}\n";

        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_shared_code, expected_shared_code, "shader code differs");
        assert_eq!(actual_code, expected_code, "unique code differs");
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

        let name = UniqueSdfClassName::new("test".to_string());

        let (actual_name, actual_code, generator_under_test) = generate_code(tree.clone(), name.clone());

        let mut actual_shared_code = String::new();
        generator_under_test.generate_shared_code(&mut actual_shared_code);

        let expected_name = FunctionName::from(&name);
        let expected_code = "fn sdf_test(point: vec3f, time: f32) -> f32 {\nvar left_3: f32;\n{\nleft_3 = sdf_test_1(point,time);\n}\nvar right_3: f32;\n{\nvar left_2: f32;\n{\nleft_2 = sdf_test_2(point,time);\n}\nvar right_2: f32;\n{\nright_2 = sdf_test_2(point,time);\n}\n\nright_3 = min(left_2,right_2);\n}\n\nreturn min(left_3,right_3);\n}\n";
        let expected_shared_code = "fn sdf_test_1(point: vec3f, time: f32) -> f32 {\nreturn length(point)-17.0;\n}\nfn sdf_test_2(point: vec3f, time: f32) -> f32 {\nvar left: f32;\n{\nleft = sdf_test_1(point,time);\n}\nvar right: f32;\n{\nright = sdf_test_1(point,time);\n}\n\nreturn min(left,right);\n}\n";

        assert_eq!(actual_name, expected_name);
        assert_eq!(actual_shared_code, expected_shared_code, "shared code differs");
        assert_eq!(actual_code, expected_code, "unique code differs");
    }

    #[must_use]
    fn generate_code(sdf: Rc<dyn Sdf>, name: UniqueSdfClassName) -> (FunctionName, String, SdfCodeGenerator) {
        let named = NamedSdf::new(sdf, name);
        let mut registrator_under_test = SdfRegistrator::default();

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

    fn assert_no_shared_code(system_under_test: SdfCodeGenerator) {
        let mut buffer = String::new();
        system_under_test.generate_shared_code(&mut buffer);
        assert!(buffer.is_empty());
    }

    #[must_use]
    fn make_dummy_sdf() -> Rc<dyn Sdf> {
        SdfSphere::new(17.0)
    }
}