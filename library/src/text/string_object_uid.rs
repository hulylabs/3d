#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default, )]
pub struct StringObjectUid(u32);

impl From<u32> for StringObjectUid {
    fn from(value: u32) -> Self {
        StringObjectUid(value)
    }
}
