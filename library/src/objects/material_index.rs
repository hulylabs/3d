#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MaterialIndex(pub usize);

impl From<usize> for MaterialIndex {
    #[must_use]
    fn from(value: usize) -> Self {
        MaterialIndex(value)
    }
}