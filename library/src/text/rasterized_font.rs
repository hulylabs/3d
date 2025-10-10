use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
use crate::utils::bitmap_utils::BitmapSize;
use cgmath::Vector2;

pub(super) struct Kerning {
    first: char,
    second: char,
    amount: f64,
}

impl Kerning {
    #[must_use]
    pub(super) fn new(first: char, second: char, amount: f64) -> Self {
        Self { first, second, amount }
    }

    #[must_use]
    pub(super) fn first(&self) -> char {
        self.first
    }

    #[must_use]
    pub(super) fn second(&self) -> char {
        self.second
    }

    #[must_use]
    pub(super) fn amount(&self) -> f64 {
        self.amount
    }
}

pub(crate) struct GlyphUvLocation {
    uv: Vector2<f64>,
    size: Vector2<f64>,
}

impl GlyphUvLocation {
    #[must_use]
    pub(super) fn new(x: usize, y: usize, width: usize, height: usize, canvas: BitmapSize) -> Self {
        let u = canvas.normalize_by_width_usize(x);
        let v = canvas.normalize_by_height_usize(y);
        let width = canvas.normalize_by_width_usize(width);
        let height = canvas.normalize_by_height_usize(height);

        Self { uv: Vector2::new(u, v), size: Vector2::new(width, height), }
    }

    #[must_use]
    pub(super) fn size(&self) -> Vector2<f64> {
        self.size
    }
}

impl GpuSerializationSize for GlyphUvLocation {
    const SERIALIZED_QUARTET_COUNT: usize = 1;
}

impl GpuSerializable for GlyphUvLocation {
    fn serialize_into(&self, buffer: &mut GpuReadySerializationBuffer) {
        buffer.write_quartet_f64(self.uv.x, self.uv.y, self.size.x, self.size.y)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct Glyph {
    gpu_buffer_index: u32,
    size: Vector2<f64>,
    offset: Vector2<f64>,
    x_advance: f64,
}

impl Glyph {
    #[must_use]
    pub(super) fn new(index: u32, uv_location: &GlyphUvLocation, x_offset: isize, y_offset: isize, x_advance: isize, canvas: BitmapSize) -> Self {
        Glyph {
            gpu_buffer_index: index,
            size: uv_location.size,
            offset: Vector2::new(canvas.normalize_by_width_isize(x_offset), canvas.normalize_by_height_isize(y_offset)),
            x_advance: canvas.normalize_by_width_isize(x_advance),
        }
    }

    #[must_use]
    pub(crate) fn index(&self) -> u32 {
        self.gpu_buffer_index
    }

    #[must_use]
    pub(crate) fn offset(&self) -> Vector2<f64> {
        self.offset
    }

    #[must_use]
    pub(crate) fn size(&self) -> Vector2<f64> {
        self.size
    }

    #[must_use]
    pub(crate) fn x_advance(&self) -> f64 {
        self.x_advance
    }
}

pub(super) struct Char {
    value: char,
    glyph: Glyph,
    glyph_uv_location: GlyphUvLocation,
}

impl Char {
    #[must_use]
    pub(super) fn new(value: char, glyph: Glyph, glyph_uv_location: GlyphUvLocation) -> Self {
        Self { value, glyph, glyph_uv_location }
    }

    #[must_use]
    pub(super) fn consume(self) -> (char, Glyph, GlyphUvLocation) {
        (self.value, self.glyph, self.glyph_uv_location)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_equals(actual: GlyphUvLocation, expected_uv: Vector2<f64>, expected_size: Vector2<f64>) {
        let mut buffer = GpuReadySerializationBuffer::new(1, GlyphUvLocation::SERIALIZED_QUARTET_COUNT);

        actual.serialize_into(&mut buffer);

        assert!(buffer.fully_written());

        let actual_state_floats: &[f32] = bytemuck::cast_slice(buffer.backend());

        assert_eq!(actual_state_floats[0], expected_uv.x as f32);
        assert_eq!(actual_state_floats[1], expected_uv.y as f32);
        assert_eq!(actual_state_floats[2], expected_size.x as f32);
        assert_eq!(actual_state_floats[3], expected_size.y as f32);
    }

    #[test]
    fn test_glyph_uv_location_zero_offset() {
        let canvas = BitmapSize::new(100, 200);

        let system_under_test = GlyphUvLocation::new(0, 0, 10, 20, canvas);

        assert_equals(system_under_test, Vector2::new(0.0, 0.0), Vector2::new(0.1, 0.1));
    }

    #[test]
    fn test_glyph_uv_location_non_zero_offset() {
        let canvas = BitmapSize::new(200, 400);

        let system_under_test = GlyphUvLocation::new(50, 100, 25, 50, canvas);

        assert_equals(system_under_test, Vector2::new(0.25, 0.25), Vector2::new(0.125, 0.125));
    }

    #[test]
    fn test_kerning_construction() {
        let expected_first = 'A';
        let expected_second = 'B';
        let expected_amount = 7.0;

        let system_under_test = Kerning::new(expected_first, expected_second, expected_amount);

        assert_eq!(system_under_test.first(), expected_first);
        assert_eq!(system_under_test.second(), expected_second);
        assert_eq!(system_under_test.amount(), expected_amount);
    }

    #[test]
    fn test_glyph_construction() {
        let expected_index = 5_u32;
        let canvas_size = BitmapSize::new(300, 700);
        let uv_location = GlyphUvLocation::new(10, 20, 30, 140, canvas_size);
        let x_offset = 600_isize;
        let y_offset = 700_isize;
        let x_advance = 300_isize;

        let system_under_test = Glyph::new(expected_index, &uv_location, x_offset, y_offset, x_advance, canvas_size);

        assert_eq!(system_under_test.index(), expected_index);
        assert_eq!(system_under_test.offset(), Vector2::new(2.0, 1.0));
        assert_eq!(system_under_test.size(), Vector2::new(0.1, 0.2));
        assert_eq!(system_under_test.x_advance(), 1.0);
    }
}