use crate::text::rasterized_font::{Char, Glyph, GlyphUvLocation, Kerning};
use crate::utils::bitmap_utils::BitmapSize;
use anyhow::anyhow;
use roxmltree::Node;

pub(super) fn attribute_value<T: std::str::FromStr>(node: &Node, name: &str) -> anyhow::Result<T> {
    node.attribute(name)
        .and_then(|v| v.parse::<T>().ok())
        .ok_or_else(|| anyhow!("bmfont parsing: missing or invalid '{name}' attribute"))
}

pub(super) fn parse_kerning(node: &Node, canvas: BitmapSize) -> anyhow::Result<Kerning> {
    let first: u32 = attribute_value(node, "first")?;
    let second: u32 = attribute_value(node, "second")?;
    let amount: isize = attribute_value(node, "amount")?;

    let first_char = char::from_u32(first);
    let second_char = char::from_u32(second);
    if let (Some(first_char), Some(second_char)) = (first_char, second_char) {
        Ok(Kerning::new(first_char, second_char, canvas.normalize_by_width_isize(amount)))
    } else {
        Err(anyhow!("failed to convert glyph kerning pair {first},{second}, skipping it"))
    }
}

pub(super) fn parse_char(node: &Node, canvas: BitmapSize, glyph_index: u32) -> anyhow::Result<Char> {
    let id: u32 = attribute_value(node, "id")?;

    let x: usize = attribute_value(node, "x")?;
    let y: usize = attribute_value(node, "y")?;
    let width: usize = attribute_value(node, "width")?;
    let height: usize = attribute_value(node, "height")?;

    let x_offset: isize = attribute_value(node, "xoffset")?;
    let y_offset: isize = attribute_value(node, "yoffset")?;
    let x_advance: isize = attribute_value(node, "xadvance")?;

    let uv_location = GlyphUvLocation::new(x, y, width, height, canvas);

    if let Some(char) = char::from_u32(id) {
        let glyph = Glyph::new(glyph_index, &uv_location, x_offset, y_offset, x_advance, canvas);
        Ok(Char::new(char, glyph, uv_location))
    } else {
        Err(anyhow!("failed to convert glyph id {} to char, skipping it", id))
    }
}

#[cfg(test)]
mod tests {
    use roxmltree::Document;
    use super::*;
    use crate::utils::bitmap_utils::BitmapSize;

    #[must_use]
    fn create_test_canvas() -> BitmapSize {
        BitmapSize::new(300, 500)
    }

    #[must_use]
    fn create_xml_node(xml: &str) -> Document<'_> {
        Document::parse(xml).unwrap()
    }

    #[test]
    fn test_attribute_value_valid() {
        let document = create_xml_node(r#"<node value="-37"/>"#);

        let actual_value = attribute_value::<isize>(&document.root_element(), "value");
        assert_eq!(actual_value.unwrap(), -37);
    }

    #[test]
    fn test_attribute_value_missing() {
        let document = create_xml_node(r#"<node/>"#);

        let actual_value = attribute_value::<i32>(&document.root_element(), "value");
        assert!(actual_value.is_err());
    }

    #[test]
    fn test_attribute_value_invalid_type() {
        let document = create_xml_node(r#"<node value="not_a_number"/>"#);

        let actual_value = attribute_value::<i32>(&document.root_element(), "value");
        assert!(actual_value.is_err());
    }

    #[test]
    fn test_parse_kerning_valid() {
        let document = create_xml_node(r#"<kerning first="65" second="87" amount="-5"/>"#);
        let size = create_test_canvas();

        let parsed_kerning = parse_kerning(&document.root_element(), size).unwrap();

        assert_eq!(parsed_kerning.first(), 65 as char);
        assert_eq!(parsed_kerning.second(), 87 as char);
        assert_eq!(parsed_kerning.amount(), size.normalize_by_width_isize(-5));
    }

    #[test]
    fn test_parse_kerning_invalid_char() {
        let canvas = create_test_canvas();

        let document = create_xml_node(r#"<kerning first="1114112" second="87" amount="-5"/>"#);
        let parsed_kerning = parse_kerning(&document.root_element(), canvas);
        assert!(parsed_kerning.is_err());

        let document = create_xml_node(r#"<kerning first="1" second="1114112" amount="-5"/>"#);
        let parsed_kerning = parse_kerning(&document.root_element(), canvas);
        assert!(parsed_kerning.is_err());
    }

    #[test]
    fn test_parse_kerning_missing_attributes() {
        let canvas = create_test_canvas();

        let document = create_xml_node(r#"<kerning first="65" amount="-5"/>"#);
        let parsed_kerning = parse_kerning(&document.root_element(), canvas);
        assert!(parsed_kerning.is_err());

        let document = create_xml_node(r#"<kerning second="65" amount="-5"/>"#);
        let parsed_kerning = parse_kerning(&document.root_element(), canvas);
        assert!(parsed_kerning.is_err());

        let document = create_xml_node(r#"<kerning first="65" second="65"/>"#);
        let parsed_kerning = parse_kerning(&document.root_element(), canvas);
        assert!(parsed_kerning.is_err());
    }

    #[test]
    fn test_parse_char_valid() {
        let document = create_xml_node(r#"<char id="65" x="10" y="20" width="30" height="40" xoffset="-5" yoffset="-10" xadvance="35"/>"#);
        let canvas = create_test_canvas();
        let expected_index: u32 = 7;

        let parsed_char = parse_char(&document.root_element(), canvas, expected_index);
        let (char, glyph, _) = parsed_char.unwrap().consume();
        assert_eq!(char, 'A');
        assert_eq!(glyph.index(), expected_index);
        assert_eq!(glyph.x_advance(), canvas.normalize_by_width_isize(35));
        assert_eq!(glyph.size().x, canvas.normalize_by_width_usize(30));
        assert_eq!(glyph.size().y, canvas.normalize_by_height_usize(40));
        assert_eq!(glyph.offset().x, canvas.normalize_by_width_isize(-5));
        assert_eq!(glyph.offset().y, canvas.normalize_by_height_isize(-10));
    }

    #[test]
    fn test_parse_char_invalid_id() {
        let document = create_xml_node(r#"<char id="1114112" x="10" y="20" width="30" height="40" xoffset="5" yoffset="10" xadvance="35"/>"#);
        let canvas = create_test_canvas();
        
        let parsed_char = parse_char(&document.root_element(), canvas, 0);
        assert!(parsed_char.is_err());
    }

    #[test]
    fn test_parse_char_missing_attributes() {
        let document = create_xml_node(r#"<char id="65" x="10"/>"#);
        let canvas = create_test_canvas();
        
        let parsed_char = parse_char(&document.root_element(), canvas, 0);
        assert!(parsed_char.is_err());
    }
}