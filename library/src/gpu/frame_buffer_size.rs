#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct FrameBufferSize {
    width: u32,
    height: u32,
}

impl FrameBufferSize {
    #[must_use]
    pub(crate) fn new(width: u32, height: u32) -> Self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
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