use wgpu::{Sampler, Texture};
use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::resources::Resources;

pub(super) struct BitmapTextures {
    sampler: Sampler,
    atlas_page: Texture,
}

impl BitmapTextures {
    const ATLAS_SAMPLER_LABEL: &'static str = "atlases_sampler";
    const ATLAS_PAGE_LABEL: &'static str = "atlas_page";

    const ATLAS_PAGE_SIZE: u32 = 1024;

    const BIND_GROUP_SAMPLER_SLOT: u32 = 1;
    const BIND_GROUP_ATLAS_PAGE_SLOT: u32 = 2;

    #[must_use]
    pub(super) fn new(resources: &Resources) -> Self {
        Self {
            sampler: resources.create_sampler(BitmapTextures::ATLAS_SAMPLER_LABEL),
            atlas_page: resources.create_texture(BitmapTextures::ATLAS_PAGE_LABEL, 1, BitmapTextures::ATLAS_PAGE_SIZE, BitmapTextures::ATLAS_PAGE_SIZE),
        }
    }

    pub(super) fn bind(&self, bind_group: &mut BindGroupBuilder) {
        bind_group.set_sampler_entry(BitmapTextures::BIND_GROUP_SAMPLER_SLOT, self.sampler.clone());
        bind_group.set_texture_entry(BitmapTextures::BIND_GROUP_ATLAS_PAGE_SLOT, self.atlas_page.create_view(&wgpu::TextureViewDescriptor::default()));
    }
}