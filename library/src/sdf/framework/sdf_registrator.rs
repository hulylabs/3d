use crate::sdf::framework::dfs;
use crate::sdf::framework::function_body_dossier::FunctionBodyDossier;
use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
use crate::sdf::framework::shader_code_dossier::ShaderCodeDossier;
use crate::sdf::framework::stack::Stack;
use crate::shader::code::{FunctionBody, ShaderCode};
use crate::shader::function_name::FunctionName;
use crate::utils::uid_generator::UidGenerator;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
pub struct SdfUid(pub u32);

impl From<u32> for SdfUid {
    fn from(value: u32) -> Self {
        SdfUid(value)
    }
}

pub struct SdfRegistrator {
    sdf_bodies: FunctionBodyDossier,
    uid_generator: UidGenerator<SdfUid>,
    registered: HashMap<UniqueSdfClassName, NamedSdf>,
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

    #[must_use]
    pub(super) fn registrations(self) -> (FunctionBodyDossier, HashMap<UniqueSdfClassName, NamedSdf>) {
        (self.sdf_bodies, self.registered)
    }
}

impl Default for SdfRegistrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::object::sdf_sphere::SdfSphere;

    #[test]
    #[should_panic]
    fn test_identical_names_registration_attempt() {
        let name = UniqueSdfClassName::new("the_name".to_string());
        let named = NamedSdf::new(SdfSphere::new(13.0), name.clone());

        let mut registrator_under_test = SdfRegistrator::new();
        registrator_under_test.add(&named);
        registrator_under_test.add(&named);
    }
}
