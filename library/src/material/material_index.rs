#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MaterialIndex(pub usize);

impl From<usize> for MaterialIndex {
    fn from(value: usize) -> Self {
        MaterialIndex(value)
    }
}