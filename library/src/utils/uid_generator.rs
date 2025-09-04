#[derive(Default)]
pub(crate) struct UidGenerator<T> {
    last_generated_uid: u32,
    returned: Vec<T>,
}

impl<T> UidGenerator<T>
where
    T: From<u32> + Copy,
{
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            last_generated_uid: 0,
            returned: Vec::new(),
        }
    }

    #[must_use]
    pub(crate) fn next(&mut self) -> T {
        if let Some(uid) = self.returned.pop() {
            uid
        } else {
            self.last_generated_uid = self.last_generated_uid.wrapping_add(1);
            T::from(self.last_generated_uid)
        }
    }

    pub(crate) fn put_back(&mut self, uid: T) {
        self.returned.push(uid);
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::object_uid::ObjectUid;
    use super::*;

    #[test]
    fn test_uid_generator_creates_new_uids() {
        let mut system_under_test = UidGenerator::<ObjectUid>::new();

        let first = system_under_test.next();
        let second = system_under_test.next();

        assert_ne!(first, second);
    }

    #[test]
    fn test_uid_generator_put_back_and_reuse() {
        let mut system_under_test = UidGenerator::<ObjectUid>::new();

        let first = system_under_test.next();
        system_under_test.put_back(first);
        let second = system_under_test.next();

        assert_eq!(first, second);
    }
}