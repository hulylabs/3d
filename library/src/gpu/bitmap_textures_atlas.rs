use std::rc::Rc;
use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::bitmap_texture::{BitmapSlot, BitmapTexture, SamplerSlot};
use crate::gpu::resources::Resources;
use crate::utils::bitmap_utils::{BitmapSize, BitmapsAtlas};
use crate::utils::version::Version;

pub(super) struct BitmapTexturesAtlas {
    atlas_page: BitmapTexture,
    last_seen_data_version: Option<Version<BitmapsAtlas>>,
}

impl BitmapTexturesAtlas {
    const SAMPLER_LABEL: &'static str = "textures_atlas_sampler";
    const PAGE_LABEL: &'static str = "textures_atlas_page";

    const BIND_GROUP_SAMPLER_SLOT: SamplerSlot = SamplerSlot(1);
    const BIND_GROUP_ATLAS_PAGE_SLOT: BitmapSlot = BitmapSlot(2);

    #[must_use]
    pub(super) fn new(resources: &Resources, atlas_page_size: BitmapSize) -> Self {
        let sampler = Rc::new(resources.create_texture_atlas_sampler(BitmapTexturesAtlas::SAMPLER_LABEL));
        Self {
            atlas_page: BitmapTexture::new(resources, sampler, Self::BIND_GROUP_SAMPLER_SLOT, atlas_page_size, Self::BIND_GROUP_ATLAS_PAGE_SLOT, Self::PAGE_LABEL),
            last_seen_data_version: None,
        }
    }

    pub(super) fn bind(&self, bind_group: &mut BindGroupBuilder) {
        self.atlas_page.bind(bind_group);
    }
    
    pub(super) fn set_atlas_page(&mut self, resources: &Resources, data: &[u8], data_version: Option<Version<BitmapsAtlas>>) {
        self.atlas_page.set_bitmap(resources, data);
        self.last_seen_data_version = data_version;
    }

    #[must_use]
    pub(super) fn last_seen_data_version(&self) -> Option<Version<BitmapsAtlas>> {
        self.last_seen_data_version
    }
}