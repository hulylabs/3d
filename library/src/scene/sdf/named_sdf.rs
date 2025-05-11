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