use std::collections::HashMap;
use std::rc::Rc;
use disjoint::DisjointSetVec;
use crate::scene::sdf::shader_code_dossier::ShaderCodeDossier;
use crate::scene::sdf::sdf::Sdf;

pub(super) struct EqualitySets {
    equality: DisjointSetVec<Rc<dyn Sdf>>, 
    index_of: HashMap<*const dyn Sdf, usize>,
}

impl EqualitySets {
    #[must_use]
    pub(super) fn new(support: &Vec<&ShaderCodeDossier>) -> Self {
        let capacity = support.iter().map(|item| item.occurrences()).sum();
        let mut equality = DisjointSetVec::<Rc<dyn Sdf>>::with_capacity(capacity);
        let mut index_of = HashMap::<*const dyn Sdf, usize>::with_capacity(capacity);
        
        for dossier in support.iter() {
            let representative = equality.push(dossier.source());
            index_of.insert(dossier.source().as_ref(), representative);
            
            for source in &dossier.sources()[1..] {
                let new_insertion_index = equality.push(source.clone());
                index_of.insert(source.as_ref(), new_insertion_index);
                equality.join(representative, new_insertion_index);
            }
        }
        
        Self { equality, index_of, }
    }
    
    #[must_use]
    pub(super) fn get_equality_root(&self, representative: Rc<dyn Sdf>) -> Rc<dyn Sdf> {
        let representative_index = self.index_of.get(&Rc::as_ptr(&representative)).unwrap();
        let root_index = self.equality.root_of(*representative_index);
        self.equality[root_index].clone()
    }
}