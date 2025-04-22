use crate::utils::object_uid::ObjectUid;

#[derive(Default)]
pub(crate) struct UidGenerator {
    last_generated_uid: u32,
    returned: Vec<ObjectUid>,
}

impl UidGenerator {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { last_generated_uid: 0, returned: Vec::new() }
    }

    #[must_use]
    pub(crate) fn next(&mut self) -> ObjectUid {
        if self.returned.is_empty() {
            self.last_generated_uid = self.last_generated_uid.wrapping_add(1);
            
            ObjectUid(self.last_generated_uid)
        } else {
            
            self.returned.remove(self.returned.len() - 1)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn put_back(&mut self, uid: ObjectUid) {
        self.returned.push(uid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uid_generator_creates_new_uids() {
        let mut system_under_test = UidGenerator::new();

        let first = system_under_test.next();
        let second = system_under_test.next();

        assert_ne!(first, second);
    }

    #[test]
    fn test_uid_generator_put_back_and_reuse() {
        let mut system_under_test = UidGenerator::new();

        let first = system_under_test.next();
        system_under_test.put_back(first);
        let second = system_under_test.next();

        assert_eq!(first, second);
    }
}