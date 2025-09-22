#[cfg(test)]
pub(crate) mod tests {
    use crate::utils::file_system::ensure_folders_exist;
    use image::{ImageBuffer, Rgb, RgbImage};
    use std::fmt::{Display, Formatter};
    use std::path::Path;

    #[must_use]
    pub(crate) fn make_png_file_name(test_case_name: &str) -> String {
        format!("{}.png", test_case_name)
    }

    pub(crate) struct ComparisonResult {
        min_diff_distance: f32,
        max_diff_distance: f32,
        different_pixels_count: usize,
        dimensions_are_same: bool,
    }

    impl ComparisonResult {
        #[must_use]
        fn new(dimensions_are_same: bool) -> Self {
            Self {
                min_diff_distance: f32::INFINITY,
                max_diff_distance: f32::NEG_INFINITY,
                different_pixels_count: 0,
                dimensions_are_same,
            }
        }

        #[must_use]
        fn diff_is_zero(&self) -> bool {
            self.different_pixels_count == 0
        }

        fn register(&mut self, left: &Rgb<u8>, right: &Rgb<u8>) {
            let length: f32 = Self::diff_length(left, right);
            if 0.0 < length {
                self.min_diff_distance = length.min(self.min_diff_distance);
                self.max_diff_distance = length.max(self.max_diff_distance);
                self.different_pixels_count += 1;
            }
        }

        #[must_use]
        pub(crate) fn are_same(&self) -> bool {
            self.dimensions_are_same && self.diff_is_zero()
        }

        #[must_use]
        fn diff_length(left: &Rgb<u8>, right: &Rgb<u8>) -> f32 {
            let difference_r = left[0] as f32 - right[0] as f32;
            let difference_g = left[1] as f32 - right[1] as f32;
            let difference_b = left[2] as f32 - right[2] as f32;
            (difference_r * difference_r + difference_g * difference_g + difference_b * difference_b).sqrt()
        }
    }

    impl Display for ComparisonResult {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            if self.dimensions_are_same {
                let min_distance = if self.min_diff_distance == f32::INFINITY { 0.0 } else { self.min_diff_distance };
                formatter.write_str(
                    format!(
                        "different pixels count: {}, min distance: {}, max distance: {}",
                        self.different_pixels_count, min_distance, self.max_diff_distance
                    )
                    .as_str(),
                )
            } else {
                formatter.write_str("dimensions are different")
            }
        }
    }

    pub(crate) fn compare_png_images<P: AsRef<Path>>(left_image_path: P, right_image_path: P, diff_output_path: P) -> Result<ComparisonResult, Box<dyn std::error::Error>> {
        let left = image::open(&left_image_path)?.to_rgb8();
        let right = image::open(&right_image_path)?.to_rgb8();

        if left.dimensions() != right.dimensions() {
            println!("images have different dimensions: {:?} vs {:?}", left.dimensions(), right.dimensions());
            return Ok(ComparisonResult::new(false));
        }

        let (width, height) = left.dimensions();
        let mut comparison_result = ComparisonResult::new(true);

        for (left_pixel, right_pixel) in left.pixels().zip(right.pixels()) {
            if left_pixel != right_pixel {
                comparison_result.register(left_pixel, right_pixel);
            }
        }

        if comparison_result.diff_is_zero() {
            return Ok(comparison_result);
        }

        let mut diff_image: RgbImage = ImageBuffer::new(width, height);

        for (x, y, left_pixel) in left.enumerate_pixels() {
            let right_pixel = right.get_pixel(x, y);

            if left_pixel != right_pixel {
                let diff_r = (left_pixel[0] as i16 - right_pixel[0] as i16).abs() as u8;
                let diff_g = (left_pixel[1] as i16 - right_pixel[1] as i16).abs() as u8;
                let diff_b = (left_pixel[2] as i16 - right_pixel[2] as i16).abs() as u8;

                // amplify differences for better visibility
                let amplified_r = std::cmp::min(255, diff_r.saturating_mul(5));
                let amplified_g = std::cmp::min(255, diff_g.saturating_mul(5));
                let amplified_b = std::cmp::min(255, diff_b.saturating_mul(5));

                diff_image.put_pixel(x, y, Rgb([amplified_r, amplified_g, amplified_b]));
            } else {
                diff_image.put_pixel(x, y, Rgb([32, 32, 32]));
            }
        }

        ensure_folders_exist(&diff_output_path)?;
        diff_image.save(&diff_output_path)?;
        println!("difference image saved to: {:?}", diff_output_path.as_ref());
        Ok(comparison_result)
    }

    #[cfg(test)]
    mod comparison_result_tests {
        use super::*;
        use rstest::rstest;

        #[test]
        fn test_register_identical_pixels() {
            let mut system_under_test = ComparisonResult::new(true);
            let left = Rgb([100, 150, 200]);
            let right = Rgb([100, 150, 200]);

            system_under_test.register(&left, &right);

            assert!(system_under_test.diff_is_zero());
            assert!(system_under_test.are_same());
        }

        #[rstest]
        #[case(101, 100, 100)]
        #[case(101, 100, 100)]
        #[case(101, 100, 100)]
        fn test_register_different_pixels(#[case] r: u8, #[case] g: u8, #[case] b: u8) {
            let mut system_under_test = ComparisonResult::new(true);
            let base_pixel = Rgb([100, 100, 100]);
            let different_pixel = Rgb([r, g, b]);

            system_under_test.register(&base_pixel, &different_pixel);

            assert_eq!(system_under_test.diff_is_zero(), false);
            assert_eq!(system_under_test.are_same(), false);
        }

        #[test]
        fn test_register_multiple_pixels() {
            let mut system_under_test = ComparisonResult::new(true);

            system_under_test.register(&Rgb([0, 0, 0]), &Rgb([10, 0, 0]));
            system_under_test.register(&Rgb([0, 0, 0]), &Rgb([0, 0, 0]));
            system_under_test.register(&Rgb([100, 100, 100]), &Rgb([103, 100, 100]));
            system_under_test.register(&Rgb([0, 0, 0]), &Rgb([0, 15, 0]));

            assert!(format!("{}", system_under_test).contains("3"));
            assert_eq!(system_under_test.diff_is_zero(), false);
            assert_eq!(system_under_test.are_same(), false);
        }

        #[test]
        fn test_construction_with_different_dimensions() {
            let system_under_test = ComparisonResult::new(false);
            assert_eq!(system_under_test.are_same(), false);
            assert!(system_under_test.diff_is_zero());
        }

        #[test]
        fn test_construction_with_same_dimensions() {
            let system_under_test = ComparisonResult::new(true);
            assert!(system_under_test.are_same());
            assert!(system_under_test.diff_is_zero());
        }
    }
}
