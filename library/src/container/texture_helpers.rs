use crate::container::texture_atlas_page_composer::{AtlasRegionUid, TextureAtlasPageComposer};
use crate::utils::bitmap_utils::{BitmapSize, ImmutableBitmapReference};
use anyhow::anyhow;
use std::path::PathBuf;

pub fn load_bitmap(file_path: PathBuf, composer: &mut TextureAtlasPageComposer) -> anyhow::Result<AtlasRegionUid> {
    let image = image::open(&file_path).map_err(|e| anyhow!("failed to open image {:?}: {}", file_path, e))?;

    let buffer = image.to_rgba8();
    let bitmap_size = BitmapSize::new(buffer.width() as usize, buffer.height() as usize);

    composer
        .allocate(ImmutableBitmapReference::new(buffer.as_raw(), bitmap_size))
        .ok_or_else(|| anyhow!("failed to allocate region in texture atlas for {:?}", file_path))
}
