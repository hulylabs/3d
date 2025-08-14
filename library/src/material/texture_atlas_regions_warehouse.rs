use crate::material::atlas_region_mapping::AtlasRegionMapping;
use crate::material::atlas_region_mapping_uid::AtlasRegionMappingUid;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::serialize_batch;
use crate::utils::uid_generator::UidGenerator;
use crate::utils::version::Version;
use std::collections::HashMap;
use crate::material::bitmap_texture_index::BitmapTextureIndex;

pub(crate) struct TextureAtlasRegionsWarehouse {
    data_version: Version,
    uid_generator: UidGenerator<AtlasRegionMappingUid>,
    index_from_uid: HashMap<AtlasRegionMappingUid, usize>,
    regions: Vec<AtlasRegionMapping>,
}

impl TextureAtlasRegionsWarehouse {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            data_version: Version(0),
            uid_generator: UidGenerator::new(),
            index_from_uid: HashMap::new(),
            regions: Vec::new(),
        }
    }

    #[must_use]
    pub(crate) fn add_region(&mut self, region: AtlasRegionMapping) -> AtlasRegionMappingUid {
        let uid = self.uid_generator.next();
        self.regions.push(region);
        self.index_from_uid.insert(uid, self.regions.len() - 1);
        self.data_version += 1;
        uid
    }

    #[must_use]
    pub(crate) fn get_region_index(&self, uid: AtlasRegionMappingUid) -> Option<BitmapTextureIndex> {
        self.index_from_uid.get(&uid).map(|i| BitmapTextureIndex(*i+1))
    }

    #[must_use]
    pub(crate) fn version(&self) -> Version {
        self.data_version
    }

    #[must_use]
    pub(crate) fn serialize(&self) -> GpuReadySerializationBuffer {
        serialize_batch(&self.regions)
    }

    #[must_use]
    pub(crate) fn count(&self) -> usize {
        self.regions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::atlas_region_mapping::AtlasRegionMappingBuilder;
    use cgmath::{Vector2, Zero};
    use crate::material::texture_region::TextureRegion;

    #[test]
    fn test_add() {
        let mut system_under_test = TextureAtlasRegionsWarehouse::new();
        let version_zero = system_under_test.version();

        let region_one = AtlasRegionMappingBuilder::new().build(TextureRegion::new(Vector2::zero(), Vector2::new(1.0, 1.0)));
        let region_two = AtlasRegionMappingBuilder::new().build(TextureRegion::new(Vector2::zero(), Vector2::new(1.0, 1.0)));

        let uid_one = system_under_test.add_region(region_one.clone());
        let version_one = system_under_test.version();

        let uid_two = system_under_test.add_region(region_two.clone());
        let version_two = system_under_test.version();

        assert_ne!(uid_one, uid_two, "UIDs should be unique for different regions");

        assert_ne!(version_zero, version_one, "versions should differ after adding a region");
        assert_ne!(version_one, version_two, "versions should differ after adding a region");
    }

    #[test]
    fn test_get_region_index_unknown_uid() {
        let mut system_under_test = TextureAtlasRegionsWarehouse::new();

        let uid = system_under_test.add_region(AtlasRegionMappingBuilder::new().build(TextureRegion::new(Vector2::zero(), Vector2::new(1.0, 1.0))));
        let index = system_under_test.get_region_index(uid).unwrap();
        assert_eq!(index, BitmapTextureIndex(0), "index should be 0 for the single region");

        let unknown_uid = AtlasRegionMappingUid(999);
        assert_eq!(
            system_under_test.get_region_index(unknown_uid),
            None,
            "unknown UID should return None"
        );
    }
}