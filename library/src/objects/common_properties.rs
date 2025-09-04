use crate::material::material_index::MaterialIndex;
pub(crate) use crate::utils::object_uid::ObjectUid;

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

    pub fn set_material_index(&mut self, new_material: MaterialIndex) {
        self.material_index = new_material;
    }
}
