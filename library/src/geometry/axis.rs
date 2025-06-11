use std::mem;
use strum::EnumCount;
use strum_macros::EnumCount;

#[derive(EnumCount, Copy, Clone, Default, Debug, PartialEq)]
#[repr(usize)]
pub enum Axis {
    #[default]
    X,
    Y,
    Z,
}

impl Axis {
    #[must_use]
    pub(crate) fn next(self) -> Axis {
        unsafe { mem::transmute((self as usize + 1) % Axis::COUNT) }
    }

    #[must_use]
    pub const fn as_index(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next() {
        assert_eq!(Axis::X.next(), Axis::Y);
        assert_eq!(Axis::Y.next(), Axis::Z);
        assert_eq!(Axis::Z.next(), Axis::X);
    }
}