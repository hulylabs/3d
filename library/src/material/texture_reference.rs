use crate::material::bitmap_texture_index::BitmapTextureIndex;
use crate::material::procedural_texture_index::ProceduralTextureUid;

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub enum TextureReference {
    Procedural(ProceduralTextureUid),
    Bitmap(BitmapTextureIndex),
    #[default]
    None,
}

type GpuSideIndexType = i32;

impl TextureReference {
    #[must_use]
    pub(crate) fn as_gpu_readable_index(&self) -> GpuSideIndexType {
        match self {
            TextureReference::Procedural(code_block_index) => {
                match i32::try_from(code_block_index.0) {
                    Ok(x) => -x,
                    Err(_) => panic!("index is too big: can't safely convert to negative i32"),
                }
            }
            TextureReference::Bitmap(bitmap_index) => {
                i32::try_from(bitmap_index.0).expect("index is too big: can't safely convert to i32")
            }
            TextureReference::None => {
                0
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::bitmap_texture_index::BitmapTextureIndex;
    use crate::material::procedural_texture_index::ProceduralTextureUid;

    #[test]
    fn test_default() {
        let system_under_test = TextureReference::default();
        assert_eq!(system_under_test, TextureReference::None);
    }

    #[test]
    fn test_none_as_gpu_readable_index() {
        let system_under_test = TextureReference::None;
        assert_eq!(system_under_test.as_gpu_readable_index(), 0);
    }

    #[test]
    fn test_bitmap_as_gpu_readable_index() {
        let bitmap_index = BitmapTextureIndex(42);
        let system_under_test = TextureReference::Bitmap(bitmap_index);
        assert_eq!(system_under_test.as_gpu_readable_index(), 42);
    }

    #[test]
    fn test_procedural_as_gpu_readable_index() {
        let procedural_index = ProceduralTextureUid(17);
        let system_under_test = TextureReference::Procedural(procedural_index);
        assert_eq!(system_under_test.as_gpu_readable_index(), -17);
    }
    
    #[test]
    fn test_bitmap_max_safe_index() {
        let bitmap_index = BitmapTextureIndex(i32::MAX as usize);
        let system_under_test = TextureReference::Bitmap(bitmap_index);
        assert_eq!(system_under_test.as_gpu_readable_index(), i32::MAX);
    }

    #[test]
    fn test_procedural_max_safe_index() {
        let procedural_index = ProceduralTextureUid(i32::MAX as usize);
        let system_under_test = TextureReference::Procedural(procedural_index);
        assert_eq!(system_under_test.as_gpu_readable_index(), -i32::MAX);
    }

    #[test]
    #[should_panic]
    fn test_bitmap_overflow_panic() {
        let bitmap_index = BitmapTextureIndex(usize::MAX);
        let system_under_test = TextureReference::Bitmap(bitmap_index);
        let _ = system_under_test.as_gpu_readable_index();
    }

    #[test]
    #[should_panic]
    fn test_procedural_overflow_panic() {
        let procedural_index = ProceduralTextureUid(usize::MAX);
        let system_under_test = TextureReference::Procedural(procedural_index);
        let _ = system_under_test.as_gpu_readable_index();
    }

    #[test]
    fn test_copy_clone_traits() {
        let system_under_test = TextureReference::Bitmap(BitmapTextureIndex(5));
        let copied = system_under_test;
        let cloned = system_under_test.clone();

        assert_eq!(system_under_test, copied);
        assert_eq!(system_under_test, cloned);
        assert_eq!(copied, cloned);
    }
    
    #[test]
    fn test_debug_trait() {
        let bitmap = TextureReference::Bitmap(BitmapTextureIndex(42));
        let procedural = TextureReference::Procedural(ProceduralTextureUid(10));
        let none = TextureReference::None;

        // ensure Debug formatting doesn't panic and produces some output
        assert!(!format!("{:?}", bitmap).is_empty());
        assert!(!format!("{:?}", procedural).is_empty());
        assert!(!format!("{:?}", none).is_empty());
    }
}