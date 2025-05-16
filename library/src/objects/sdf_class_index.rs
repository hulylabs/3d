use derive_more::Display;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Display)]
pub(crate) struct SdfClassIndex(pub usize);

impl SdfClassIndex {
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0 as f64
    }
}

impl From<usize> for SdfClassIndex {
    #[must_use]
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