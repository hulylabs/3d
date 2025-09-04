use crate::utils::version::Version;

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

    pub(super) fn delete_object(&mut self) {
        assert!(self.object_count > 0);
        self.object_count -= 1;
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

    pub(super) fn clear_objects(&mut self) {
        self.object_count = 0;
        self.data_version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_context::{test_context, TestContext};

    struct Context {
        system_under_test: Statistics
    }

    impl TestContext for Context {
        fn setup() -> Context {
            Context {  system_under_test: Statistics::default() }
        }

        fn teardown(self) {
        }
    }

    #[test_context(Context)]
    #[test]
    fn test_initial_statistics(fixture: &mut Context) {
        assert_eq!(fixture.system_under_test.data_version(), Version(0));
        assert_eq!(fixture.system_under_test.object_count(), 0);
    }

    #[test_context(Context)]
    #[test]
    fn test_delete_object(fixture: &mut Context) {
        fixture.system_under_test.register_new_object();
        let version_before_deletion = fixture.system_under_test.data_version();
        fixture.system_under_test.delete_object();
        assert_eq!(fixture.system_under_test.object_count(), 0);
        assert_ne!(fixture.system_under_test.data_version(), version_before_deletion);
    }

    #[test_context(Context)]
    #[test]
    fn test_register_new_object(fixture: &mut Context) {
        fixture.system_under_test.register_new_object();
        assert_eq!(fixture.system_under_test.object_count(), 1);
        assert_eq!(fixture.system_under_test.data_version(), Version(1));

        fixture.system_under_test.register_new_object();
        assert_eq!(fixture.system_under_test.object_count(), 2);
        assert_eq!(fixture.system_under_test.data_version(), Version(2));
    }

    #[test_context(Context)]
    #[test]
    fn test_register_object_mutation(fixture: &mut Context) {
        fixture.system_under_test.register_new_object();
        assert_eq!(fixture.system_under_test.object_count(), 1);
        assert_eq!(fixture.system_under_test.data_version(), Version(1));

        fixture.system_under_test.register_object_mutation();
        assert_eq!(fixture.system_under_test.object_count(), 1);
        assert_eq!(fixture.system_under_test.data_version(), Version(2));

        fixture.system_under_test.register_new_object();
        assert_eq!(fixture.system_under_test.object_count(), 2);
        assert_eq!(fixture.system_under_test.data_version(), Version(3));
    }

    #[test_context(Context)]
    #[test]
    fn test_multiple_mutations(fixture: &mut Context) {
        fixture.system_under_test.register_new_object();
        assert_eq!(fixture.system_under_test.object_count(), 1);
        assert_eq!(fixture.system_under_test.data_version(), Version(1));

        fixture.system_under_test.register_object_mutation();
        fixture.system_under_test.register_object_mutation();
        assert_eq!(fixture.system_under_test.object_count(), 1);
        assert_eq!(fixture.system_under_test.data_version(), Version(3));
    }
}
