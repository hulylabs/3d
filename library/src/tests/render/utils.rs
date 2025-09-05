#[cfg(test)]
pub(crate) mod tests {
    use std::path::Path;
    use image::{ImageBuffer, Rgb, RgbImage};
    use crate::utils::file_system::ensure_folders_exist;

    #[must_use]
    pub(crate) fn make_png_file_name(test_case_name: &str) -> String {
        format!("{}.png", test_case_name)
    }

    pub(crate) fn compare_png_images<P: AsRef<Path>>(
        left_image_path: P,
        right_image_path: P,
        diff_output_path: P,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let left = image::open(&left_image_path)?.to_rgb8();
        let right = image::open(&right_image_path)?.to_rgb8();

        if left.dimensions() != right.dimensions() {
            println!("images have different dimensions: {:?} vs {:?}", left.dimensions(), right.dimensions()
            );
            return Ok(false);
        }

        let (width, height) = left.dimensions();

        let mut are_identical = true;
        for (left_pixel, right_pixel) in left.pixels().zip(right.pixels()) {
            if left_pixel != right_pixel {
                are_identical = false;
                break;
            }
        }

        if are_identical {
            return Ok(true);
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
        Ok(false)
    }
}