use more_asserts::{assert_ge, assert_gt};

pub(crate) const BYTES_IN_RGBA_QUARTET: usize = 4;

#[derive(Clone, Copy)]
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
    pub(crate) fn size(&self) -> &BitmapSize {
        &self.size
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

pub(crate) fn write_sub_bitmap(
    container: MutableBitmapReference,
    sub_bitmap: ImmutableBitmapReference,
    destination_u: usize,
    destination_v: usize,
) {
    assert_ge!(container.size.width, destination_u + sub_bitmap.size.width, "sub-bitmap horizontal extends beyond atlas bounds");
    assert_ge!(container.size.height, destination_v + sub_bitmap.size.height, "sub-bitmap vertical extends beyond atlas bounds");

    let mut container = container;
    
    for row in 0..sub_bitmap.size.height {
        let container_row = destination_v + row;
        let container_row_start = container_row * container.size.width * BYTES_IN_RGBA_QUARTET;
        let container_pixel_start = container_row_start + destination_u * BYTES_IN_RGBA_QUARTET;
        let container_pixel_end = container_pixel_start + sub_bitmap.size.width * BYTES_IN_RGBA_QUARTET;

        let sub_row_start = row * sub_bitmap.size.width * BYTES_IN_RGBA_QUARTET;
        let sub_row_end = sub_row_start + sub_bitmap.size.width * BYTES_IN_RGBA_QUARTET;

        let source = &sub_bitmap.data()[sub_row_start..sub_row_end];
        container.data_mut()[container_pixel_start..container_pixel_end].copy_from_slice(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[must_use]
    fn write_single_pixel(container_size: BitmapSize, pixel: &[u8; BYTES_IN_RGBA_QUARTET], u: usize, v: usize) -> Vec<u8> {
        let mut container: Vec<u8> = vec![0u8; container_size.bytes_in_bitmap()];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(pixel, BitmapSize::new(1, 1)),
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
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap = vec![
            // Row 0
            255, 0, 0, 255,
            0, 255, 0, 255,
            // Row 1
            0, 0, 255, 255,
            255, 255, 0, 255,
        ];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_bitmap, BitmapSize::new(2, 2)),
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
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap = vec![
            255, 0, 0, 255,
            0, 255, 0, 255,
            0, 0, 255, 255,
            255, 255, 255, 255,
        ];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_bitmap, BitmapSize::new(2, 2)),
            0, 0,
        );

        assert_eq!(container, sub_bitmap);
    }

    #[test]
    fn test_1x1_atlas() {
        let container_size = BitmapSize::new(1, 1);
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap = vec![255, 128, 64, 32];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_bitmap, BitmapSize::new(1, 1)),
            0, 0,
        );

        assert_eq!(container, sub_bitmap);
    }

    #[test]
    #[should_panic]
    fn test_sub_image_extends_beyond_right_edge() {
        let container_size = BitmapSize::new(3, 3);
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap_size = BitmapSize::new(2, 1); // 2x1 image
        let sub_image = vec![255u8; sub_bitmap_size.bytes_in_bitmap()];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_image, sub_bitmap_size),
            2, 0,
        );
    }

    #[test]
    #[should_panic]
    fn test_sub_image_extends_beyond_bottom_edge() {
        let container_size = BitmapSize::new(3, 3);
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap_size = BitmapSize::new(1, 2); // 2x1 image
        let sub_image = vec![255u8; sub_bitmap_size.bytes_in_bitmap()]; // 1x2 image

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_image, sub_bitmap_size),
            0, 2,
        );
    }

    #[test]
    fn system_under_test_handles_horizontal_stripe() {
        let container_size = BitmapSize::new(4, 2);
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap_size = BitmapSize::new(3, 1);// 3 wide, 1 tall
        let sub_bitmap = vec![
            255, 0, 0, 255,   // red
            0, 255, 0, 255,   // green
            0, 0, 255, 255,   // blue
        ];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_bitmap, sub_bitmap_size),
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
        let mut container = vec![0u8; container_size.bytes_in_bitmap()];
        let sub_bitmap_size = BitmapSize::new(1, 3); // 1 wide, 3 tall
        let sub_bitmap = vec![
            255, 0, 0, 255,     // red (row 0)
            0, 255, 0, 255,     // green (row 1)
            0, 0, 255, 255,     // blue (row 2)
        ];

        write_sub_bitmap(
            MutableBitmapReference::new(&mut container, container_size),
            BitmapReference::new(&sub_bitmap, sub_bitmap_size),
            0, 1,
        );

        let red_offset = (1 * 2 + 0) * BYTES_IN_RGBA_QUARTET; // row 1, col 0
        assert_eq!(container[red_offset..red_offset + 4], [255, 0, 0, 255]);

        let green_offset = (2 * 2 + 0) * BYTES_IN_RGBA_QUARTET; // row 2, col 0
        assert_eq!(container[green_offset..green_offset + 4], [0, 255, 0, 255]);

        let blue_offset = (3 * 2 + 0) * BYTES_IN_RGBA_QUARTET; // row 3, col 0
        assert_eq!(container[blue_offset..blue_offset + 4], [0, 0, 255, 255]);
    }
}