use wgpu::{Sampler, Texture};
use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::resources::Resources;
use crate::utils::bitmap_utils::BitmapSize;
use crate::utils::version::Version;

pub(super) struct BitmapTextures {
    sampler: Sampler,
    atlas_page: Texture,
    last_seen_data_version: Option<Version>,
}

impl BitmapTextures {
    const ATLAS_SAMPLER_LABEL: &'static str = "atlases_sampler";
    const ATLAS_PAGE_LABEL: &'static str = "atlas_page";

    const BIND_GROUP_SAMPLER_SLOT: u32 = 1;
    const BIND_GROUP_ATLAS_PAGE_SLOT: u32 = 2;

    #[must_use]
    pub(super) fn new(resources: &Resources, atlas_page_size: BitmapSize) -> Self {
        Self {
            sampler: resources.create_sampler(BitmapTextures::ATLAS_SAMPLER_LABEL),
            atlas_page: resources.create_texture(BitmapTextures::ATLAS_PAGE_LABEL, 1, atlas_page_size),
            last_seen_data_version: None,
        }
    }

    pub(super) fn bind(&self, bind_group: &mut BindGroupBuilder) {
        bind_group.set_sampler_entry(BitmapTextures::BIND_GROUP_SAMPLER_SLOT, self.sampler.clone());
        bind_group.set_texture_entry(BitmapTextures::BIND_GROUP_ATLAS_PAGE_SLOT, self.atlas_page.create_view(&wgpu::TextureViewDescriptor::default()));
    }
    
    pub(super) fn set_atlas_page(&mut self, resources: &Resources, data: &[u8], data_version: Option<Version>) {
        resources.write_whole_srgba_texture_data(&self.atlas_page, data);
        self.last_seen_data_version = data_version;
    }

    #[must_use]
    pub(super) fn last_seen_data_version(&self) -> Option<Version> {
        self.last_seen_data_version
    }
}