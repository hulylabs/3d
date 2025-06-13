use std::ops::{Add, AddAssign};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub(crate) struct Version(pub u64);

impl AddAssign<u64> for Version {
    fn add_assign(&mut self, right: u64) {
        self.0 += right;
    }
}

impl Add<u64> for Version {
    type Output = Self;

    #[must_use]
    fn add(self, other: u64) -> Self {
        Version(self.0 + other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let initial = Version(5);
        let increased = initial + 3;
        
        assert_eq!(increased, Version(8));
        assert_eq!(initial, Version(5));
    }

    #[test]
    fn test_add_assign() {
        let mut system_under_test = Version(10);
        system_under_test += 7;
        assert_eq!(system_under_test, Version(17));
    }

    #[test]
    fn test_default() {
        let system_under_test = Version::default();
        assert_eq!(system_under_test, Version(0));
    }
}