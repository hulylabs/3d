use std::marker::PhantomData;
use std::ops::{Add, AddAssign};

pub(crate) struct Version<T> {
    value: u64,
    _phantom: PhantomData<T>,
}

impl<T> Copy for Version<T> {
}

impl<T> Clone for Version<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> std::fmt::Debug for Version<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Version")
            .field("value", &self.value)
            .finish()
    }
}

impl<T> PartialEq for Version<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Version<T> {
}

impl<T> PartialOrd for Version<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Version<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T> Version<T> {
    #[must_use]
    pub(crate) fn new(version: u64) -> Self {
        Self { value: version, _phantom: PhantomData }
    }
}

impl<T> Default for Version<T> {
    fn default() -> Self {
        Version::new(0)
    }
}

impl<T> AddAssign<u64> for Version<T> {
    fn add_assign(&mut self, right: u64) {
        self.value += right;
    }
}

impl<T> Add<u64> for Version<T> {
    type Output = Self;

    fn add(self, other: u64) -> Self {
        Version::new(self.value + other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Example types to version
    struct User;
    struct Document;

    #[test]
    fn test_add() {
        let initial = Version::<User>::new(5);
        let increased = initial + 3;
        
        assert_eq!(increased, Version::<User>::new(8));
        assert_eq!(initial, Version::<User>::new(5));
    }

    #[test]
    fn test_add_assign() {
        let mut system_under_test = Version::<Document>::new(10);
        system_under_test += 7;
        assert_eq!(system_under_test, Version::<Document>::new(17));
    }

    #[test]
    fn test_default() {
        let system_under_test = Version::<User>::default();
        assert_eq!(system_under_test, Version::<User>::new(0));
    }
}