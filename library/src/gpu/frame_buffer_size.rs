use cgmath::{Vector2, Vector3};

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FrameBufferSize {
    width: u32,
    height: u32,
}

impl FrameBufferSize {
    #[must_use]
    pub(crate) const fn new(width: u32, height: u32) -> Self {
        assert!(width > 0);
        assert!(height > 0);
        Self { width, height }
    }

    #[must_use]
    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub(crate) fn area(&self) -> u32 {
        self.width * self.height
    }
    
    #[must_use]
    pub(crate) fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
    
    #[must_use]
    pub(crate) fn work_groups_count(&self, work_group_size: Vector2<u32>) -> Vector3<u32> {
        Vector3::<u32>::new(
            self.width.div_ceil(work_group_size.x), 
            self.height.div_ceil(work_group_size.y),
            1
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_groups_count_no_reminder() {
        let expected_width: u32 = 37;
        let expected_height: u32 = 43;
        let width_multiplier: u32 = 3;
        let height_multiplier: u32 = 4;
        let system_under_test = FrameBufferSize::new(expected_width * width_multiplier, expected_height * height_multiplier);

        let actual_count = system_under_test.work_groups_count(Vector2::new(expected_width, expected_height));
        let expected_count = Vector3::new(width_multiplier, height_multiplier, 1u32);
        
        assert_eq!(actual_count, expected_count);
    }
    
    #[test]
    fn test_work_groups_count_with_reminder() {
        let expected_width: u32 = 37;
        let expected_height: u32 = 43;
        let width_multiplier: u32 = 3;
        let height_multiplier: u32 = 4;
        let system_under_test = FrameBufferSize::new(expected_width * width_multiplier + 5, expected_height * height_multiplier + 7);

        let actual_count = system_under_test.work_groups_count(Vector2::new(expected_width, expected_height));
        let expected_count = Vector3::new(width_multiplier + 1, height_multiplier + 1, 1u32);
        
        assert_eq!(actual_count, expected_count);
    }

    #[test]
    fn test_aspect() {
        let width: u32 = 1000;
        let height: u32 = 500;
        let system_under_test = FrameBufferSize::new(width, height);
        
        assert_eq!(system_under_test.aspect(), (width as f32) / (height as f32));
    }
    
    #[test]
    fn test_construction() {
        let expected_width = 1920;
        let expected_height = 1080;
        let system_under_test = FrameBufferSize::new(expected_width, expected_height);
        assert_eq!(system_under_test.width(), expected_width);
        assert_eq!(system_under_test.height(), expected_height);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_zero_width() {
        let _system_under_test = FrameBufferSize::new(0, 1080);
    }

    #[test]
    #[should_panic(expected = "assertion failed")]
    fn test_zero_height() {
        let _system_under_test = FrameBufferSize::new(1920, 0);
    }

    #[test]
    fn test_area() {
        let width = 1920;
        let height = 1080;
        let system_under_test = FrameBufferSize::new(width, height);
        let expected_area = width * height;

        assert_eq!(system_under_test.area(), expected_area);
    }
}