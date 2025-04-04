#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MaterialIndex(pub usize);

impl MaterialIndex {
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}

impl From<usize> for MaterialIndex {
    #[must_use]
    fn from(value: usize) -> Self {
        MaterialIndex(value)
    }
}