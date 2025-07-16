use derive_more::Display;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Display)]
pub(crate) struct SdfClassIndex(pub usize);

impl SdfClassIndex {
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self.0 as i32
    }
}

impl From<usize> for SdfClassIndex {
    fn from(value: usize) -> Self {
        SdfClassIndex(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    
    #[test]
    fn test_display() {
        let mut buffer = String::new();
        write!(buffer, "{}", SdfClassIndex(17)).unwrap();
        assert_eq!(buffer, "17");
    }
}