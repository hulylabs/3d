use crate::objects::material_index::MaterialIndex;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ObjectUid(pub u32);
impl From<usize> for ObjectUid {
    fn from(value: usize) -> Self {
        ObjectUid(value as u32)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Linkage {
    uid: ObjectUid,
    material_index: MaterialIndex,
}

impl Linkage {
    #[must_use]
    pub(crate) const fn new(uid: ObjectUid, material_index: MaterialIndex,) -> Self {
        Linkage {
            uid,
            material_index,
        }
    }

    #[must_use]
    pub(crate) const fn uid(self) -> ObjectUid {
        self.uid
    }
    
    #[must_use]
    pub(crate) const fn material_index(self) -> MaterialIndex {
        self.material_index
    }
}
