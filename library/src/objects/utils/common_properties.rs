#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct GlobalObjectIndex(pub u32);
impl GlobalObjectIndex {
    #[must_use]
    pub(crate) const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct MaterialIndex(pub u32);
impl MaterialIndex {
    #[must_use]
    pub(crate) const fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct Linkage<InKindObjectIndex: Copy>
{
    global_index: GlobalObjectIndex,
    in_kind_index: InKindObjectIndex,
    material_index: MaterialIndex,
}

impl<InKindObjectIndex: Copy> Linkage<InKindObjectIndex> {
    #[must_use]
    pub(crate) const fn new(global_index: GlobalObjectIndex, in_kind_index: InKindObjectIndex, material_index: MaterialIndex,) -> Self {
        Linkage {
            global_index,
            in_kind_index,
            material_index,
        }
    }

    #[must_use]
    pub(crate) const fn global_index(&self) -> GlobalObjectIndex {
        self.global_index
    }

    #[must_use]
    pub(crate) const fn in_kind_index(&self) -> InKindObjectIndex {
        self.in_kind_index
    }

    #[must_use]
    pub(crate) const fn material_index(&self) -> MaterialIndex {
        self.material_index
    }
}
