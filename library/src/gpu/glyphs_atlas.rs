use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::bitmap_texture::{BitmapSlot, BitmapTexture, SamplerSlot};
use crate::gpu::resources::Resources;
use crate::utils::bitmap_utils::BitmapSize;
use std::rc::Rc;

pub(super) struct GlyphsAtlas {
    atlas: BitmapTexture,
}

impl GlyphsAtlas {
    const ATLAS_SAMPLER_LABEL: &'static str = "glyphs_atlas_sampler";
    const ATLAS_LABEL: &'static str = "glyphs_atlas";

    const BIND_GROUP_SAMPLER_SLOT: SamplerSlot = SamplerSlot(3);
    const BIND_GROUP_ATLAS_SLOT: BitmapSlot = BitmapSlot(4);

    #[must_use]
    pub(super) fn new(resources: &Resources, atlas_page_size: BitmapSize) -> Self {
        let sampler = Rc::new(resources.create_glyph_atlas_sampler(Self::ATLAS_SAMPLER_LABEL));
        Self {
            atlas: BitmapTexture::new(resources, sampler, Self::BIND_GROUP_SAMPLER_SLOT, atlas_page_size, Self::BIND_GROUP_ATLAS_SLOT, Self::ATLAS_LABEL),
        }
    }

    pub(super) fn bind(&self, bind_group: &mut BindGroupBuilder) {
        self.atlas.bind(bind_group);
    }

    pub(super) fn set_atlas_page(&mut self, resources: &Resources, data: &[u8]) {
        self.atlas.set_bitmap(resources, data);
    }
}