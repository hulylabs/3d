use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::resources::Resources;
use crate::utils::bitmap_utils::BitmapSize;
use std::rc::Rc;
use wgpu::{Sampler, Texture};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash,)]
pub(super) struct SamplerSlot(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash,)]
pub(super) struct BitmapSlot(pub u32);

pub(super) struct BitmapTexture {
    sampler: Rc<Sampler>,
    bind_group_sampler_slot: SamplerSlot,

    bitmap: Texture,
    bind_group_bitmap_slot: BitmapSlot,
}

impl BitmapTexture {
    #[must_use]
    pub(super) fn new(resources: &Resources, sampler: Rc<Sampler>, bind_group_sampler_slot: SamplerSlot, size: BitmapSize, bind_group_bitmap_slot: BitmapSlot, label: &str) -> Self {
        Self {
            sampler,
            bind_group_sampler_slot,
            bitmap: resources.create_texture(label, 1, size),
            bind_group_bitmap_slot,
        }
    }

    pub(super) fn bind(&self, bind_group: &mut BindGroupBuilder) {
        bind_group.set_sampler_entry(self.bind_group_sampler_slot.0, (*self.sampler).clone());
        bind_group.set_texture_entry(self.bind_group_bitmap_slot.0, self.bitmap.create_view(&wgpu::TextureViewDescriptor::default()));
    }

    pub(super) fn set_bitmap(&mut self, resources: &Resources, data: &[u8]) {
        resources.write_whole_srgba_texture_data(&self.bitmap, data);
    }
}

