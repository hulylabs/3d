use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
use crate::text::font_bits_parsing::{attribute_value, parse_char, parse_kerning};
use crate::text::rasterized_font::{Glyph, GlyphUvLocation};
use crate::utils::bitmap_utils::BitmapSize;
use anyhow::anyhow;
use cgmath::Vector2;
use image::RgbaImage;
use log::warn;
use roxmltree::Document;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub(super) struct GlyphAtlasComposer {
    layout_serialized: GpuReadySerializationBuffer,
    atlas: RgbaImage,
    atlas_bitmap_size: BitmapSize,
    max_glyph_uv_size: Option<Vector2<f64>>,
    char_to_glyph_index: HashMap<char, Glyph>,
    generation_time_distance_range: f64,
    kernings: HashMap<char, HashMap<char, f64>>,
    line_height: f64,
    line_baseline: f64,
}

impl GlyphAtlasComposer {
    const BMFONT_FILE_EXTENSION: &'static str = "fnt";

    pub(crate) fn new(atlas_bitmap_file: impl AsRef<Path>,) -> anyhow::Result<Self> {
        let atlas_bitmap_file_path = atlas_bitmap_file.as_ref();
        let atlas_layout_file_path = atlas_bitmap_file_path.with_extension(Self::BMFONT_FILE_EXTENSION);
        
        let image = image::open(atlas_bitmap_file_path).map_err(|e| anyhow!("failed to open image {}: {}", atlas_bitmap_file_path.display(), e))?;

        let atlas = image.to_rgba8();
        let atlas_bitmap_size = BitmapSize::new(atlas.width() as usize, atlas.height() as usize);

        let atlas_layout_content = fs::read_to_string(atlas_layout_file_path)?;
        let parsed = parse_bmfont_xml(atlas_layout_content.as_str(), atlas_bitmap_size)?;

        let mut layout_serialized = GpuReadySerializationBuffer::new(parsed.gpu_data.len(), GlyphUvLocation::SERIALIZED_QUARTET_COUNT);
        for item in parsed.gpu_data {
            item.serialize_into(&mut layout_serialized);
        }
        assert!(layout_serialized.fully_written(), "not all glyphs have been serialized");

        Ok(Self {
            layout_serialized,
            atlas,
            atlas_bitmap_size,
            max_glyph_uv_size: parsed.max_glyph_uv_size,
            char_to_glyph_index: parsed.char_to_glyph,
            generation_time_distance_range: parsed.generation_time_distance_range,
            kernings: parsed.chars_kerning,
            line_height: parsed.line_height,
            line_baseline: parsed.line_baseline,
        })
    }

    #[must_use]
    pub(crate) fn get_atlas_bitmap(&self) -> &Vec<u8> {
        self.atlas.as_raw()
    }

    #[must_use]
    pub(crate) fn get_atlas_layout(&self) -> GpuReadySerializationBuffer {
        self.layout_serialized.clone()
    }

    #[must_use]
    pub(crate) fn get_glyph(&self, char: char) -> Option<Glyph> {
        self.char_to_glyph_index.get(&char).copied()
    }

    #[must_use]
    pub(crate) fn get_glyph_kerning(&self, current: char, previous: char) -> Option<f64> {
        self.kernings.get(&current).and_then(|map| map.get(&previous)).copied()
    }

    #[must_use]
    pub(crate) fn get_atlas_size(&self) -> BitmapSize {
        self.atlas_bitmap_size
    }

    #[must_use]
    pub(crate) fn generation_time_distance_range(&self) -> f64 {
        self.generation_time_distance_range
    }

    #[must_use]
    pub(crate) fn line_baseline(&self) -> f64 {
        self.line_baseline
    }

    #[must_use]
    pub(crate) fn line_height(&self) -> f64 {
        self.line_height
    }

    #[must_use]
    pub(crate) fn max_glyph_uv_size(&self) -> Option<Vector2<f64>> {
        self.max_glyph_uv_size
    }

    #[must_use]
    pub(crate) fn make_empty_atlas_layout_buffer() -> GpuReadySerializationBuffer {
        GpuReadySerializationBuffer::make_filled(1, GlyphUvLocation::SERIALIZED_QUARTET_COUNT, 0.0)
    }
}

#[must_use]
fn componentwise_max(left: Option<Vector2<f64>>, right: Vector2<f64>) -> Option<Vector2<f64>> {
    if let Some(left) = left {
        Some(Vector2::new(left.x.max(right.x), left.y.max(right.y)))
    } else {
        Some(right)
    }
}

struct Glyphs {
    char_to_glyph: HashMap<char, Glyph>,
    chars_kerning: HashMap<char, HashMap<char, f64>>,
    gpu_data: Vec<GlyphUvLocation>,
    max_glyph_uv_size: Option<Vector2<f64>>,
    generation_time_distance_range: f64,
    line_height: f64,
    line_baseline: f64,
}

impl Glyphs {
    #[must_use]
    fn new() -> Self {
        Self {
            char_to_glyph: HashMap::new(),
            gpu_data: Vec::new(),
            max_glyph_uv_size: None,
            generation_time_distance_range: 1.0,
            chars_kerning: HashMap::new(),
            line_height: 0.0,
            line_baseline: 0.0,
        }
    }
}

fn parse_bmfont_xml(xml_content: &str, atlas_bitmap_size: BitmapSize) -> anyhow::Result<Glyphs> {
    let mut result = Glyphs::new();
    let mut glyph_index: u32 = 0;

    let doc = Document::parse(xml_content)?;
    for node in doc.descendants() {
        if node.has_tag_name("common") {
            let line_height: usize = attribute_value(&node, "lineHeight")?;
            result.line_height = atlas_bitmap_size.normalize_by_height_usize(line_height);
            let line_baseline: usize = attribute_value(&node, "base")?;
            result.line_baseline = atlas_bitmap_size.normalize_by_height_usize(line_baseline);
        } else if node.has_tag_name("distanceField") {
            result.generation_time_distance_range = attribute_value(&node, "distanceRange")?;
        } else if node.has_tag_name("kerning") {
            match parse_kerning(&node, atlas_bitmap_size) {
                Ok(kerning) => {
                    let predecessors = result.chars_kerning.entry(kerning.second()).or_default();
                    predecessors.insert(kerning.first(), kerning.amount());
                }
                Err(error) => {
                    warn!("failed to parse kerning pair: {}; skipping it", error);
                }
            }
        } else if node.has_tag_name("char") {
            match parse_char(&node, atlas_bitmap_size, glyph_index) {
                Ok(char) => {
                    let (char_value, glyph, uv_location) = char.consume();
                    result.max_glyph_uv_size = componentwise_max(result.max_glyph_uv_size, uv_location.size());
                    result.char_to_glyph.insert(char_value, glyph);
                    result.gpu_data.push(uv_location);
                    glyph_index += 1;
                }
                Err(error) => {
                    warn!("failed to parse char: {}; skipping it", error);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::tribal_knowledge::tests::ASCII_PRINTABLE_GLYPHS_COUNT;
    use crate::utils::tests::conventions::tests::data_folder_path;
    use more_asserts::assert_ge;
    use std::collections::HashSet;

    #[test]
    fn test_componentwise_max_with_none() {
        let expected = Vector2::new(3.0, 5.0);

        assert_eq!(componentwise_max(None, expected), Some(expected));
    }

    #[test]
    fn test_componentwise_max_with_some_left_greater() {
        let left = Vector2::new(7.0, 8.0);
        let right = Vector2::new(3.0, 5.0);

        assert_eq!(componentwise_max(Some(left), right), Some(Vector2::new(left.x, left.y)));
    }

    #[test]
    fn test_componentwise_max_with_some_right_greater() {
        let left = Vector2::new(2.0, 3.0);
        let right = Vector2::new(5.0, 7.0);

        assert_eq!(componentwise_max(Some(left), right), Some(Vector2::new(right.x, right.y)));
    }

    #[test]
    fn test_componentwise_max_with_mixed_components() {
        let left = Vector2::new(7.0, 3.0);
        let right = Vector2::new(4.0, 8.0);
        assert_eq!(componentwise_max(Some(left), right), Some(Vector2::new(left.x, right.y)));

        let left = Vector2::new(2.0, 9.0);
        let right = Vector2::new(4.0, 8.0);
        assert_eq!(componentwise_max(Some(left), right), Some(Vector2::new(right.x, left.y)));
    }

    #[test]
    fn test_componentwise_max_with_equal_values() {
        let value = 5.0;
        let left = Vector2::new(value, value);
        let right = Vector2::new(value, value);

        assert_eq!(componentwise_max(Some(left), right), Some(Vector2::new(value, value)));
    }

    #[test]
    fn test_glyph_atlas_composer_ascii_atlas() {
        let system_under_test = GlyphAtlasComposer::new(data_folder_path().join("glyphs_atlas_ascii_domine.png")).unwrap();

        let expected_size = BitmapSize::new(1024, 512);
        assert_eq!(system_under_test.get_atlas_size(), expected_size);
        assert_eq!(system_under_test.get_atlas_bitmap().len(), expected_size.bytes_in_bitmap());

        assert_eq!(system_under_test.max_glyph_uv_size(), Some(Vector2::new(0.1201171875, 0.236328125)));

        assert_eq!(system_under_test.line_height(), expected_size.normalize_by_height_isize(130));
        assert_eq!(system_under_test.line_baseline(), expected_size.normalize_by_height_isize(103));
        assert_eq!(system_under_test.generation_time_distance_range(), 4.0);

        let mut indices = HashSet::<u32>::new();

        for ascii_code in ' '..='~' {
            let glyph = system_under_test.get_glyph(ascii_code);
            if let Some(glyph) = glyph {
                indices.insert(glyph.index());
                assert_ge!(glyph.x_advance(), 0.0);

                assert_ge!(glyph.offset().x.abs(), 0.0);
                assert_ge!(glyph.offset().y.abs(), 0.0);
            } else {
                panic!("failed to find glyph for ascii code {}", ascii_code);
            }
        }

        assert_eq!(indices.len(), ASCII_PRINTABLE_GLYPHS_COUNT);
    }

    #[test]
    fn test_glyph_atlas_composer_absent_glyph_fetch() {
        let system_under_test = GlyphAtlasComposer::new(data_folder_path().join("glyphs_atlas_ascii_domine.png")).unwrap();
        let non_ascii_symbol = 'Ё';
        let actual_glyph = system_under_test.get_glyph(non_ascii_symbol);
        assert_eq!(actual_glyph, None);
    }
}