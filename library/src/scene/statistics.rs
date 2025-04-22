use crate::scene::version::Version;

#[derive(Default, Copy, Clone)]
pub(super) struct Statistics {
    data_version: Version,
    object_count: usize,
}

impl Statistics {
    pub(super) fn register_new_object(&mut self) {
        self.object_count += 1;
        self.data_version += 1;
    }
    
    pub(super) fn register_object_mutation(&mut self) {
        self.data_version += 1;
    }

    #[must_use]
    pub(super) fn data_version(&self) -> Version {
        self.data_version
    }

    #[must_use]
    pub(super) fn object_count(&self) -> usize {
        self.object_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_statistics() {
        let system_under_test = Statistics::default();
        assert_eq!(system_under_test.data_version(), Version(0));
        assert_eq!(system_under_test.object_count(), 0);
    }

    #[test]
    fn test_register_new_object() {
        let mut system_under_test = Statistics::default();
        
        system_under_test.register_new_object();
        assert_eq!(system_under_test.object_count(), 1);
        assert_eq!(system_under_test.data_version(), Version(1));
        
        system_under_test.register_new_object();
        assert_eq!(system_under_test.object_count(), 2);
        assert_eq!(system_under_test.data_version(), Version(2));
    }

    #[test]
    fn test_register_object_mutation() {
        let mut system_under_test = Statistics::default();
        
        system_under_test.register_new_object();
        assert_eq!(system_under_test.object_count(), 1);
        assert_eq!(system_under_test.data_version(), Version(1));

        system_under_test.register_object_mutation();
        assert_eq!(system_under_test.object_count(), 1);
        assert_eq!(system_under_test.data_version(), Version(2));

        system_under_test.register_new_object();
        assert_eq!(system_under_test.object_count(), 2);
        assert_eq!(system_under_test.data_version(), Version(3));
    }

    #[test]
    fn test_multiple_mutations() {
        let mut system_under_test = Statistics::default();
        
        system_under_test.register_new_object();
        assert_eq!(system_under_test.object_count(), 1);
        assert_eq!(system_under_test.data_version(), Version(1));

        system_under_test.register_object_mutation();
        system_under_test.register_object_mutation();
        assert_eq!(system_under_test.object_count(), 1);
        assert_eq!(system_under_test.data_version(), Version(3));
    }
}