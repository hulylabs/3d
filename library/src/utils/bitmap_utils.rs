use cgmath::Vector2;
use derive_more::Display;
use image::{ImageBuffer, Rgba};
use more_asserts::assert_gt;
use std::path::Path;

pub(crate) const BYTES_IN_RGBA_QUARTET: usize = 4;

#[derive(Clone, Copy, Display)]
#[display("{}x{}", width, height)]
pub struct BitmapSize {
    width: usize,
    height: usize,
}

impl BitmapSize {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        assert_gt!(width * height, 0, "bitmap area can't be zero");
        Self { width, height }
    }
    #[must_use]
    pub(crate) fn width(&self) -> usize {
        self.width
    }
    #[must_use]
    pub(crate) fn height(&self) -> usize {
        self.height
    }

    #[must_use]
    pub(crate) const fn bytes_in_bitmap(&self) -> usize {
        self.width * self.height * BYTES_IN_RGBA_QUARTET
    }

    #[must_use]
    fn inside(&self, u: usize, v: usize) -> bool {
        u < self.width && v < self.height
    }
}

pub struct BitmapReference<T> {
    data: T,
    size: BitmapSize,
}

impl<T: AsRef<[u8]>> BitmapReference<T> {
    pub fn new(data: T, size: BitmapSize) -> Self {
        assert_eq!(data.as_ref().len(), size.bytes_in_bitmap(), "size mismatch");
        Self { data, size }
    }

    #[must_use]
    pub(crate) fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    #[must_use]
    pub(crate) fn size(&self) -> BitmapSize {
        self.size
    }
}

impl<T: AsMut<[u8]>> BitmapReference<T> {
    #[must_use]
    pub(crate) fn data_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

pub type ImmutableBitmapReference<'a> = BitmapReference<&'a [u8]>;
pub(crate) type MutableBitmapReference<'a> = BitmapReference<&'a mut [u8]>;

fn copy_bitmap_region(
    destination: &mut MutableBitmapReference,
    destination_texel: Vector2<usize>,
    source: &ImmutableBitmapReference,
    source_texel: Vector2<usize>,
    copy_size: BitmapSize,
) {
    assert!(source.size.inside(source_texel.x + copy_size.width - 1, source_texel.y + copy_size.height - 1), "source region exceeds source size");
    assert!(destination.size.inside(destination_texel.x + copy_size.width - 1, destination_texel.y + copy_size.height - 1), "destination region exceeds destination size");

    let bytes_per_pixel = BYTES_IN_RGBA_QUARTET;

    for row in 0..copy_size.height {
        let src_row_start = ((source_texel.y + row) * source.size.width + source_texel.x) * bytes_per_pixel;
        let src_row_end = src_row_start + copy_size.width * bytes_per_pixel;

        let dst_row_start = ((destination_texel.y + row) * destination.size.width + destination_texel.x) * bytes_per_pixel;
        let dst_row_end = dst_row_start + copy_size.width * bytes_per_pixel;

        let source_slice = &source.data()[src_row_start..src_row_end];
        destination.data_mut()[dst_row_start..dst_row_end].copy_from_slice(source_slice);
    }
}

pub(crate) fn write_sub_bitmap(
    container: &mut MutableBitmapReference,
    sub_bitmap: &ImmutableBitmapReference,
    destination_u: usize,
    destination_v: usize,
) {
    copy_bitmap_region(
        container,
        Vector2::new(destination_u, destination_v),
        sub_bitmap,
        Vector2::new(0, 0),
        sub_bitmap.size,
    );
}

pub(crate) fn write_sub_bitmap_column(
    container: &mut MutableBitmapReference,
    destination_column: usize,
    destination_row: usize,
    sub_bitmap: &ImmutableBitmapReference,
    source_column: usize,
) {
    copy_bitmap_region(
        container,
        Vector2::new(destination_column, destination_row),
        sub_bitmap,
        Vector2::new(source_column, 0),
        BitmapSize::new(1, sub_bitmap.size.height),
    );
}

pub(crate) fn write_sub_bitmap_row(
    container: &mut MutableBitmapReference,
    destination_column: usize,
    destination_row: usize,
    sub_bitmap: &ImmutableBitmapReference,
    source_row: usize,
) {
    copy_bitmap_region(
        container,
        Vector2::new(destination_column, destination_row),
        sub_bitmap,
        Vector2::new(0, source_row),
        BitmapSize::new(sub_bitmap.size.width, 1),
    );
}

#[must_use]
fn texel_byte_index(u: usize, v: usize, size: BitmapSize) -> usize {
    (v * size.width + u) * BYTES_IN_RGBA_QUARTET
}

pub(crate) fn set_texel(
    destination: &mut MutableBitmapReference,
    to_u: usize,
    to_v: usize,
    source: &ImmutableBitmapReference,
    from_u: usize,
    from_v: usize,
) {
    assert!(destination.size().inside(to_u, to_v), "destination [u,v] ({}, {}) out of bounds {}", to_u, to_v, destination.size());
    assert!(source.size().inside(from_u, from_v), "source [u,v] ({}, {}) out of bounds {}", to_u, to_v, source.size());

    let source_start = texel_byte_index(from_u, from_v, source.size());
    let source_end = source_start + BYTES_IN_RGBA_QUARTET;
    let source_texel = &source.data()[source_start..source_end];

    let destination_start = texel_byte_index(to_u, to_v, destination.size());
    let destination_end = destination_start + BYTES_IN_RGBA_QUARTET;
    destination.data_mut()[destination_start..destination_end].copy_from_slice(source_texel);
}

pub(crate) fn save_bitmap_to_png<FilePath: AsRef<Path>>(data: &[u8], size: BitmapSize, path: FilePath) -> Result<(), Box<dyn std::error::Error>> {
    let buffer = ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(
        size.width as u32,
        size.height as u32,
        data,
    ).ok_or("invalid buffer size for dimensions")?;

    buffer.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[must_use]
    fn allocate_bitmap_of_size(size: BitmapSize, filler: u8) -> Vec<u8> {
        vec![filler; size.bytes_in_bitmap()]
    }

    #[must_use]
    fn write_single_pixel(container_size: BitmapSize, pixel: &[u8; BYTES_IN_RGBA_QUARTET], u: usize, v: usize) -> Vec<u8> {
        let mut container: Vec<u8> = allocate_bitmap_of_size(container_size, 0);

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(pixel, BitmapSize::new(1, 1)),
            u, v,
        );

        container
    }

    #[test]
    fn test_write_single_pixel_at_origin() {
        let sub_bitmap: [u8; BYTES_IN_RGBA_QUARTET] = [255, 128, 64, 32];

        let container = write_single_pixel(BitmapSize::new(2, 2), &sub_bitmap, 0, 0);

        assert_eq!(container[0..4], sub_bitmap);
        assert_eq!(container[4..], [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_write_single_pixel_at_center() {
        let sub_bitmap: [u8; BYTES_IN_RGBA_QUARTET] = [255, 128, 64, 32];

        let container = write_single_pixel(BitmapSize::new(3, 3), &sub_bitmap, 1, 1);

        let center_pixel_offset = (1 * 3 + 1) * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[center_pixel_offset..center_pixel_offset + 4], sub_bitmap);

        for i in 0..9 {
            if 4 == i {
                continue;
            }
            assert_eq!(container[i*BYTES_IN_RGBA_QUARTET..(i+1)*BYTES_IN_RGBA_QUARTET], [0, 0, 0, 0]);
        }
    }

    #[test]
    fn test_write_2x2_sub_image() {
        let container_size = BitmapSize::new(4, 4);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap = vec![
            // Row 0
            255, 0, 0, 255,
            0, 255, 0, 255,
            // Row 1
            0, 0, 255, 255,
            255, 255, 0, 255,
        ];

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_bitmap, BitmapSize::new(2, 2)),
            1, 1,
        );

        // check red pixel at (1,1)
        let red_offset = (1 * 4 + 1) * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[red_offset..red_offset + 4], [255, 0, 0, 255]);

        // check green pixel at (1,2)
        let green_offset = (1 * 4 + 2) * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[green_offset..green_offset + 4], [0, 255, 0, 255]);

        // check blue pixel at (2,1)
        let blue_offset = (2 * 4 + 1) * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[blue_offset..blue_offset + 4], [0, 0, 255, 255]);

        // check yellow pixel at (2,2)
        let yellow_offset = (2 * 4 + 2) * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[yellow_offset..yellow_offset + 4], [255, 255, 0, 255]);
    }

    #[test]
    fn test_write_full_atlas() {
        let container_size = BitmapSize::new(2, 2);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap = vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
            255, 255, 255, 255,
        ];

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_bitmap, BitmapSize::new(2, 2)),
            0, 0,
        );

        assert_eq!(container, sub_bitmap);
    }

    #[test]
    fn test_1x1_atlas() {
        let container_size = BitmapSize::new(1, 1);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap = vec![255, 128, 64, 32];

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_bitmap, BitmapSize::new(1, 1)),
            0, 0,
        );

        assert_eq!(container, sub_bitmap);
    }

    #[test]
    #[should_panic]
    fn test_sub_image_extends_beyond_right_edge() {
        let container_size = BitmapSize::new(3, 3);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap_size = BitmapSize::new(2, 1);
        let sub_image = vec![255u8; sub_bitmap_size.bytes_in_bitmap()];

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_image, sub_bitmap_size),
            2, 0,
        );
    }

    #[test]
    #[should_panic]
    fn test_sub_image_extends_beyond_bottom_edge() {
        let container_size = BitmapSize::new(3, 3);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap_size = BitmapSize::new(1, 2);
        let sub_image = allocate_bitmap_of_size(sub_bitmap_size, 255u8);

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_image, sub_bitmap_size),
            0, 2,
        );
    }

    #[test]
    fn system_under_test_handles_horizontal_stripe() {
        let container_size = BitmapSize::new(4, 2);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap_size = BitmapSize::new(3, 1);
        let sub_bitmap = vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
        ];

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_bitmap, sub_bitmap_size),
            1, 0,
        );

        let red_offset = 1 * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[red_offset..red_offset + 4], [255, 0, 0, 255]);

        let green_offset = 2 * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[green_offset..green_offset + 4], [0, 255, 0, 255]);

        let blue_offset = 3 * BYTES_IN_RGBA_QUARTET;
        assert_eq!(container[blue_offset..blue_offset + 4], [0, 0, 255, 255]);
    }

    #[test]
    fn system_under_test_handles_vertical_stripe() {
        let container_size = BitmapSize::new(2, 4);
        let mut container = allocate_bitmap_of_size(container_size, 0);
        let sub_bitmap_size = BitmapSize::new(1, 3);
        let sub_bitmap = vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
        ];

        write_sub_bitmap(
            &mut MutableBitmapReference::new(&mut container, container_size),
            &BitmapReference::new(&sub_bitmap, sub_bitmap_size),
            0, 1,
        );

        let red_offset = (1 * 2 + 0) * BYTES_IN_RGBA_QUARTET; // row 1, col 0
        assert_eq!(container[red_offset..red_offset + 4], [255, 0, 0, 255]);

        let green_offset = (2 * 2 + 0) * BYTES_IN_RGBA_QUARTET; // row 2, col 0
        assert_eq!(container[green_offset..green_offset + 4], [0, 255, 0, 255]);

        let blue_offset = (3 * 2 + 0) * BYTES_IN_RGBA_QUARTET; // row 3, col 0
        assert_eq!(container[blue_offset..blue_offset + 4], [0, 0, 255, 255]);
    }

    #[test]
    fn test_write_sub_bitmap_column() {
        let source_data = vec![
            // row 0
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,

            // row 1
            255, 255, 0, 255,
            0, 255, 255, 255,
            255, 0, 255, 255,
        ];
        let source_size = BitmapSize { width: 3, height: 2 };
        let source = BitmapReference::new(source_data.as_slice(), source_size);

        let destination_size = BitmapSize { width: 4, height: 3 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0);
        let mut destination = BitmapReference::new(destination_data.as_mut_slice(), destination_size);

        write_sub_bitmap_column(&mut destination, 2, 0, &source, 1);

        let expected_green = [0, 255, 0, 255];
        let expected_cyan = [0, 255, 255, 255];

        // row 0, column 2
        assert_eq!(&destination_data[2*4..2*4+4], &expected_green);
        // row 1, column 2
        assert_eq!(&destination_data[(1*4 + 2)*4..(1*4 + 2)*4+4], &expected_cyan);

        // other pixels remain black (untouched)
        let black = [0, 0, 0, 0];
        assert_eq!(&destination_data[0..4], &black);
        assert_eq!(&destination_data[4..8], &black);
    }

    #[test]
    fn test_write_sub_bitmap_row() {
        let source_data = vec![
            // row 0
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
            // row 1
            255, 255, 0, 255,
            0, 255, 255, 255,
            255, 0, 255, 255,
        ];
        let source_size = BitmapSize { width: 3, height: 2 };
        let source = BitmapReference::new(source_data.as_slice(), source_size);

        let destination_size = BitmapSize { width: 4, height: 3 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0);
        let mut destination = BitmapReference::new(destination_data.as_mut_slice(), destination_size);

        write_sub_bitmap_row(&mut destination, 0, 1, &source, 1);

        let expected_yellow = [255, 255, 0, 255];
        let expected_cyan = [0, 255, 255, 255];
        let expected_magenta = [255, 0, 255, 255];

        let row1_start = 1 * 4 * BYTES_IN_RGBA_QUARTET;
        assert_eq!(&destination_data[row1_start..row1_start+4], &expected_yellow);
        assert_eq!(&destination_data[row1_start+4..row1_start+8], &expected_cyan);
        assert_eq!(&destination_data[row1_start+8..row1_start+12], &expected_magenta);

        let black = [0, 0, 0, 0];
        assert_eq!(&destination_data[0..4], &black);
        assert_eq!(&destination_data[2*4*4..2*4*4+4], &black);
    }

    #[test]
    #[should_panic(expected = "source region exceeds source size")]
    fn test_write_sub_bitmap_column_source_out_of_bounds() {
        let source_size = BitmapSize { width: 2, height: 2 };
        let source_data = allocate_bitmap_of_size(source_size, 0u8);

        let destination_size = BitmapSize { width: 4, height: 4 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0u8);

        write_sub_bitmap_column(
            &mut BitmapReference::new(destination_data.as_mut_slice(), destination_size), 0, 0,
            &BitmapReference::new(source_data.as_slice(), source_size), 3);
    }

    #[test]
    #[should_panic(expected = "destination region exceeds destination size")]
    fn test_write_sub_bitmap_column_dest_out_of_bounds() {
        let source_size = BitmapSize { width: 2, height: 2 };
        let source_data = allocate_bitmap_of_size(source_size, 0u8);

        let destination_size = BitmapSize { width: 4, height: 4 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0u8);

        write_sub_bitmap_column(
            &mut BitmapReference::new(destination_data.as_mut_slice(), destination_size), 4, 0,
            &BitmapReference::new(source_data.as_slice(), source_size), 0);
    }

    #[test]
    #[should_panic(expected = "source region exceeds source size")]
    fn test_write_sub_bitmap_row_source_out_of_bounds() {
        let source_size = BitmapSize { width: 2, height: 2 };
        let source_data = allocate_bitmap_of_size(source_size, 0u8);

        let destination_size = BitmapSize { width: 4, height: 4 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0u8);

        write_sub_bitmap_row(
            &mut BitmapReference::new(destination_data.as_mut_slice(), destination_size), 0, 0,
            &BitmapReference::new(source_data.as_slice(), source_size), 3);
    }

    #[test]
    #[should_panic(expected = "destination region exceeds destination size")]
    fn test_write_sub_bitmap_row_dest_out_of_bounds() {
        let source_size = BitmapSize { width: 2, height: 2 };
        let source_data = allocate_bitmap_of_size(source_size, 0u8);

        let destination_size = BitmapSize { width: 4, height: 4 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0u8);

        write_sub_bitmap_row(
            &mut BitmapReference::new(destination_data.as_mut_slice(), destination_size), 0, 4,
            &BitmapReference::new(source_data.as_slice(), source_size), 0);
    }

    #[test]
    fn test_multiple_texel_copies() {
        let source_data = vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
            255, 255, 0, 255,
        ];
        let source_size = BitmapSize { width: 2, height: 2 };
        let source = BitmapReference::new(source_data.as_slice(), source_size);

        let destination_size = BitmapSize { width: 2, height: 2 };
        let mut destination_data = allocate_bitmap_of_size(destination_size, 0u8);
        let mut destination = BitmapReference::new(destination_data.as_mut_slice(), destination_size);

        set_texel(&mut destination, 0, 0, &source, 1, 1);
        set_texel(&mut destination, 1, 0, &source, 0, 1);
        set_texel(&mut destination, 0, 1, &source, 1, 0);
        set_texel(&mut destination, 1, 1, &source, 0, 0);

        assert_eq!(&destination_data[0..4],   &[255, 255, 0,   255]);
        assert_eq!(&destination_data[4..8],   &[0,   0,   255, 255]);
        assert_eq!(&destination_data[8..12],  &[0,   255, 0,   255]);
        assert_eq!(&destination_data[12..16], &[255, 0,   0,   255]);
    }
}