use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_code_dossier::ShaderCodeDossier;
use disjoint::DisjointSetVec;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct EqualitySets {
    equality: DisjointSetVec<Rc<dyn Sdf>>, 
    index_of: HashMap<*const dyn Sdf, usize>,
}

impl EqualitySets {
    #[must_use]
    pub(crate) fn new(support: &Vec<&ShaderCodeDossier>) -> Self {
        let capacity = support.iter().map(|item| item.occurrences()).sum();
        let mut equality = DisjointSetVec::<Rc<dyn Sdf>>::with_capacity(capacity);
        let mut index_of = HashMap::<*const dyn Sdf, usize>::with_capacity(capacity);
        
        for dossier in support.iter() {
            let representative = equality.push(dossier.any_source());
            index_of.insert(dossier.any_source().as_ref(), representative);
            
            for source in &dossier.sources()[1..] {
                let new_insertion_index = equality.push(source.clone());
                index_of.insert(source.as_ref(), new_insertion_index);
                equality.join(representative, new_insertion_index);
            }
        }
        
        Self { equality, index_of, }
    }
    
    #[must_use]
    pub(crate) fn get_equality_root(&self, representative: Rc<dyn Sdf>) -> Rc<dyn Sdf> {
        let representative_index = self.index_of.get(&Rc::as_ptr(&representative)).unwrap();
        let root_index = self.equality.root_of(*representative_index);
        self.equality[root_index].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::framework::dummy_sdf::tests::make_dummy_sdf;
    use crate::sdf::framework::sdf_base::Sdf;
    use crate::sdf::framework::shader_code_dossier::ShaderCodeDossier;
    use crate::shader::function_name::FunctionName;

    #[test]
    fn test_get_equality_root() {
        let first_equality_cluster = vec![make_dummy_sdf(), make_dummy_sdf(),]; 
        let mut first_dossier = ShaderCodeDossier::new(FunctionName("first".to_string()), first_equality_cluster[0].clone(), 0);
        first_dossier.write_another_usage(first_equality_cluster[1].clone());
        
        let second_equality_cluster = vec![make_dummy_sdf(), make_dummy_sdf(), make_dummy_sdf(),];
        let mut second_dossier = ShaderCodeDossier::new(FunctionName("second".to_string()), second_equality_cluster[0].clone(), 0);
        second_dossier.write_another_usage(second_equality_cluster[1].clone());
        second_dossier.write_another_usage(second_equality_cluster[2].clone());
        
        let system_under_test = EqualitySets::new(&vec![&first_dossier, &second_dossier]);

        assert_all_equal(&first_equality_cluster, &system_under_test);
        assert_all_equal(&second_equality_cluster, &system_under_test);
        
        fn assert_all_equal(targets: &Vec<Rc<dyn Sdf>>, equality_host: &EqualitySets) {
            for sdf in targets.iter() {
                let actual_root = equality_host.get_equality_root(sdf.clone());
                let expected_root = equality_host.get_equality_root(targets[0].clone());
                assert!(std::ptr::eq(actual_root.as_ref(), expected_root.as_ref()));
            }
        }
    }
}