use cgmath::Vector2;

#[derive(Debug, Clone)]
pub struct TextureRegion {
    top_left_corner_uv: Vector2<f32>,
    size: Vector2<f32>,
}

impl TextureRegion {
    #[must_use]
    pub fn new(top_left_corner_uv: Vector2<f32>, size: Vector2<f32>) -> Self {
        assert_region_inside_unit_quad(top_left_corner_uv, size);
        Self {
            top_left_corner_uv,
            size,
        }
    }

    #[must_use]
    pub(super) fn top_left_corner_uv(&self) -> Vector2<f32> {
        self.top_left_corner_uv
    }

    #[must_use]
    pub(super) fn size(&self) -> Vector2<f32> {
        self.size
    }
}

fn assert_region_inside_unit_quad(top_left_corner_uv: Vector2<f32>, size: Vector2<f32>) {
    assert!(
        top_left_corner_uv.x >= 0.0 && top_left_corner_uv.x <= 1.0,
        "top-left corner U(x) coordinate {} is outside unit quad [0.0, 1.0]",
        top_left_corner_uv.x
    );

    assert!(
        top_left_corner_uv.y >= 0.0 && top_left_corner_uv.y <= 1.0,
        "top-left corner V(y) coordinate {} is outside unit quad [0.0, 1.0]",
        top_left_corner_uv.y
    );

    assert!(
        size.x > 0.0,
        "region width {} must be positive",
        size.x
    );
    assert!(
        size.y > 0.0,
        "region height {} must be positive",
        size.y
    );

    let bottom_right = top_left_corner_uv + size;
    assert!(
        bottom_right.x <= 1.0,
        "region extends beyond unit quad: right edge at {} exceeds 1.0",
        bottom_right.x
    );
    assert!(
        bottom_right.y <= 1.0,
        "region extends beyond unit quad: bottom edge at {} exceeds 1.0",
        bottom_right.y
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::Vector2;

    #[test]
    fn test_construction() {
        let top_left = Vector2::new(0.1, 0.2);
        let size = Vector2::new(0.3, 0.4);

        let system_under_test = TextureRegion::new(top_left, size);

        assert_eq!(system_under_test.top_left_corner_uv(), top_left);
        assert_eq!(system_under_test.size(), size);
    }

    #[test]
    #[should_panic(expected = "top-left corner U(x) coordinate -0.1 is outside unit quad [0.0, 1.0]")]
    fn test_when_top_left_x_is_negative() {
        let top_left = Vector2::new(-0.1, 0.5);
        let size = Vector2::new(0.2, 0.2);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "top-left corner V(y) coordinate -0.1 is outside unit quad [0.0, 1.0]")]
    fn test_when_top_left_y_is_negative() {
        let top_left = Vector2::new(0.5, -0.1);
        let size = Vector2::new(0.2, 0.2);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "top-left corner U(x) coordinate 1.1 is outside unit quad [0.0, 1.0]")]
    fn test_when_top_left_x_exceeds_unit_quad() {
        let top_left = Vector2::new(1.1, 0.5);
        let size = Vector2::new(0.1, 0.1);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "top-left corner V(y) coordinate 1.1 is outside unit quad [0.0, 1.0]")]
    fn test_when_top_left_y_exceeds_unit_quad() {
        let top_left = Vector2::new(0.5, 1.1);
        let size = Vector2::new(0.1, 0.1);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "region width 0 must be positive")]
    fn test_when_width_is_zero() {
        let top_left = Vector2::new(0.5, 0.5);
        let size = Vector2::new(0.0, 0.2);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "region height 0 must be positive")]
    fn test_when_height_is_zero() {
        let top_left = Vector2::new(0.5, 0.5);
        let size = Vector2::new(0.2, 0.0);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "region extends beyond unit quad: right edge at 1.1 exceeds 1.0")]
    fn test_when_region_extends_beyond_right_edge() {
        let top_left = Vector2::new(0.8, 0.5);
        let size = Vector2::new(0.3, 0.2);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    #[should_panic(expected = "region extends beyond unit quad: bottom edge at 1.1 exceeds 1.0")]
    fn test_when_region_extends_beyond_bottom_edge() {
        let top_left = Vector2::new(0.5, 0.8);
        let size = Vector2::new(0.2, 0.3);

        let _ = TextureRegion::new(top_left, size);
    }

    #[test]
    fn test_clone() {
        let top_left = Vector2::new(0.1, 0.2);
        let size = Vector2::new(0.3, 0.4);
        let original = TextureRegion::new(top_left, size);

        let system_under_test = original.clone();

        assert_eq!(system_under_test.top_left_corner_uv(), top_left);
        assert_eq!(system_under_test.size(), size);
    }

    #[test]
    fn test_debug_format() {
        let top_left = Vector2::new(0.1, 0.2);
        let size = Vector2::new(0.3, 0.4);
        let system_under_test = TextureRegion::new(top_left, size);

        let debug_string = format!("{:?}", system_under_test);

        assert!(debug_string.contains("TextureRegion"));
        assert!(debug_string.contains("0.1"));
        assert!(debug_string.contains("0.2"));
        assert!(debug_string.contains("0.3"));
        assert!(debug_string.contains("0.4"));
    }
}