#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MaterialIndex(pub usize);

impl MaterialIndex {
    #[must_use]
    pub const fn as_f32(self) -> f32 {
        self.0 as f32
    }
}

impl From<usize> for MaterialIndex {
    #[must_use]
    fn from(value: usize) -> Self {
        MaterialIndex(value)
    }
}