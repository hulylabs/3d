#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AtlasRegionUid(pub usize);

impl From<u32> for AtlasRegionUid {
    fn from(value: u32) -> Self {
        AtlasRegionUid(value as usize)
    }
}