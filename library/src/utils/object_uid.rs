#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ObjectUid(pub u32);
impl From<usize> for ObjectUid {
    #[must_use]
    fn from(value: usize) -> Self {
        ObjectUid(value as u32)
    }
}