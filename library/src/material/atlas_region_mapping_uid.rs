#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct AtlasRegionMappingUid(pub(super) usize);

impl From<u32> for AtlasRegionMappingUid {
    fn from(value: u32) -> Self {
        AtlasRegionMappingUid(value as usize)
    }
}