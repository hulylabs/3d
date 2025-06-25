use crate::material::bitmap_texture_index::BitmapTextureIndex;
use crate::material::procedural_texture_index::ProceduralTextureIndex;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TextureReference {
    Procedural(ProceduralTextureIndex),
    Bitmap(BitmapTextureIndex),
    None,
}

impl TextureReference {
    #[must_use]
    pub(crate) fn as_gpu_readable_index(&self) -> i32 {
        match self {
            TextureReference::Procedural(code_block_index) => {
                code_block_index.0 as i32
            }
            TextureReference::Bitmap(bitmap_index) => {
                bitmap_index.0 as i32
            }
            TextureReference::None => {
                0
            }
        }
    }
}

impl Default for TextureReference {
    #[must_use]
    fn default() -> Self {
        TextureReference::None
    }
}