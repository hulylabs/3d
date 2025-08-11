use crate::gpu::resizable_buffer::ResizeStatus;
use crate::gpu::versioned_buffer::BufferUpdateStatus;

pub(super) struct BuffersUpdateStatus {
    geometry_status: BufferUpdateStatus,
    materials_status: BufferUpdateStatus,
}

impl BuffersUpdateStatus {
    #[must_use]
    pub(super) fn new() -> Self {
        Self {
            geometry_status: BufferUpdateStatus::new_updated(false),
            materials_status: BufferUpdateStatus::new_updated(false),
        }
    }

    #[must_use]
    pub(super) fn any_resized(&self) -> bool {
        self.geometry_status.resized() || self.materials_status.resized()
    }

    #[must_use]
    pub(super) fn any_updated(&self) -> bool {
        self.geometry_updated() || self.materials_status.updated()
    }

    #[must_use]
    pub(super) fn geometry_updated(&self) -> bool {
        self.geometry_status.updated()
    }

    pub(super) fn merge_bvh(&mut self, child_status: ResizeStatus) {
        let updated = true;
        let resized = child_status == ResizeStatus::Resized;
        let status = BufferUpdateStatus::new(resized, updated);
        self.geometry_status = self.geometry_status.merge(status);
    }
    
    pub(super) fn merge_geometry(&mut self, child_status: BufferUpdateStatus) {
        self.geometry_status = self.geometry_status.merge(child_status);
    }

    pub(super) fn merge_materials(&mut self, child_status: BufferUpdateStatus) {
        self.materials_status = self.materials_status.merge(child_status);
    }
}

#[cfg(test)]
mod tests {
    use test_context::{test_context, TestContext};
    use super::*;

    struct Context {
        system_under_test: BuffersUpdateStatus
    }

    impl TestContext for Context {
        fn setup() -> Context {
            Context {  system_under_test: BuffersUpdateStatus::new() }
        }

        fn teardown(self) {
        }
    }

    #[test_context(Context)]
    #[test]
    fn test_construction(fixture: &mut Context) {
        assert_eq!(fixture.system_under_test.any_resized(), false);
        assert_eq!(fixture.system_under_test.any_updated(), false);
        assert_eq!(fixture.system_under_test.geometry_updated(), false);
    }

    #[test_context(Context)]
    #[test]
    fn test_geometry_only_updated(fixture: &mut Context) {
        fixture.system_under_test.merge_geometry(BufferUpdateStatus::new_updated(true));

        assert_eq!(fixture.system_under_test.geometry_updated(), true);
        assert_eq!(fixture.system_under_test.any_resized(), false);
        assert_eq!(fixture.system_under_test.any_updated(), true);
    }

    #[test_context(Context)]
    #[test]
    fn test_materials_only_updated(fixture: &mut Context) {
        fixture.system_under_test.merge_materials(BufferUpdateStatus::new_updated(true));

        assert_eq!(fixture.system_under_test.any_updated(), true);
        assert_eq!(fixture.system_under_test.any_resized(), false);
        assert_eq!(fixture.system_under_test.geometry_updated(), false);
    }

    #[test_context(Context)]
    #[test]
    fn test_geometry_and_materials_both_updated(fixture: &mut Context) {
        fixture.system_under_test.merge_geometry(BufferUpdateStatus::new_updated(true));
        fixture.system_under_test.merge_materials(BufferUpdateStatus::new_updated(true));

        assert_eq!(fixture.system_under_test.any_resized(), false);
        assert_eq!(fixture.system_under_test.any_updated(), true);
        assert_eq!(fixture.system_under_test.geometry_updated(), true);
    }

    #[test_context(Context)]
    #[test]
    fn test_geometry_resized(fixture: &mut Context) {
        fixture.system_under_test.merge_geometry(BufferUpdateStatus::new_resized(true));

        assert_eq!(fixture.system_under_test.any_resized(), true);
        assert_eq!(fixture.system_under_test.any_updated(), true);
        assert_eq!(fixture.system_under_test.geometry_updated(), true);
    }

    #[test_context(Context)]
    #[test]
    fn test_materials_resized(fixture: &mut Context) {
        fixture.system_under_test.merge_materials(BufferUpdateStatus::new_resized(true));

        assert_eq!(fixture.system_under_test.any_resized(), true);
        assert_eq!(fixture.system_under_test.any_updated(), true);
        assert_eq!(fixture.system_under_test.geometry_updated(), false);
    }

    #[test_context(Context)]
    #[test]
    fn test_geometry_and_materials_resized(fixture: &mut Context) {
        fixture.system_under_test.merge_geometry(BufferUpdateStatus::new_resized( true));
        fixture.system_under_test.merge_materials(BufferUpdateStatus::new_resized(true));

        assert_eq!(fixture.system_under_test.any_resized(), true);
        assert_eq!(fixture.system_under_test.any_updated(), true);
        assert_eq!(fixture.system_under_test.geometry_updated(), true);
    }

    #[test_context(Context)]
    #[test]
    fn test_merge_multiple_statuses(fixture: &mut Context) {
        fixture.system_under_test.merge_geometry(BufferUpdateStatus::new_resized(true));
        fixture.system_under_test.merge_geometry(BufferUpdateStatus::new_updated(true));
        fixture.system_under_test.merge_materials(BufferUpdateStatus::new_resized(true));
        fixture.system_under_test.merge_materials(BufferUpdateStatus::new_updated(true));

        assert_eq!(fixture.system_under_test.any_resized(), true);
        assert_eq!(fixture.system_under_test.any_updated(), true);
        assert_eq!(fixture.system_under_test.geometry_updated(), true);
    }

    #[test_context(Context)]
    #[test]
    fn test_merge_bvh(fixture: &mut Context) {
        fixture.system_under_test.merge_bvh(ResizeStatus::SizeKept);
        assert_eq!(fixture.system_under_test.any_resized(), false);
        assert_eq!(fixture.system_under_test.any_updated(), true);

        fixture.system_under_test.merge_bvh(ResizeStatus::Resized);
        assert_eq!(fixture.system_under_test.any_resized(), true);
        assert_eq!(fixture.system_under_test.any_updated(), true);
    }
}