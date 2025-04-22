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

    pub(super) fn merge_geometry(&mut self, child_status: BufferUpdateStatus) {
        self.geometry_status = self.geometry_status.merge(child_status);
    }

    pub(super) fn merger_material(&mut self, child_status: BufferUpdateStatus) {
        self.materials_status = self.materials_status.merge(child_status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construction() {
        let system_under_test = BuffersUpdateStatus::new();
        assert_eq!(system_under_test.any_resized(), false);
        assert_eq!(system_under_test.any_updated(), false);
        assert_eq!(system_under_test.geometry_updated(), false);
    }

    #[test]
    fn test_geometry_only_updated() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merge_geometry(BufferUpdateStatus::new_updated(true));

        assert_eq!(system_under_test.geometry_updated(), true);
        assert_eq!(system_under_test.any_resized(), false);
        assert_eq!(system_under_test.any_updated(), true);
    }

    #[test]
    fn test_materials_only_updated() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merger_material(BufferUpdateStatus::new_updated(true));

        assert_eq!(system_under_test.any_updated(), true);
        assert_eq!(system_under_test.any_resized(), false);
        assert_eq!(system_under_test.geometry_updated(), false);
    }

    #[test]
    fn test_geometry_and_materials_both_updated() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merge_geometry(BufferUpdateStatus::new_updated(true));
        system_under_test.merger_material(BufferUpdateStatus::new_updated(true));

        assert_eq!(system_under_test.any_resized(), false);
        assert_eq!(system_under_test.any_updated(), true);
        assert_eq!(system_under_test.geometry_updated(), true);
    }

    #[test]
    fn test_geometry_resized() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merge_geometry(BufferUpdateStatus::new_resized(true));

        assert_eq!(system_under_test.any_resized(), true);
        assert_eq!(system_under_test.any_updated(), true);
        assert_eq!(system_under_test.geometry_updated(), true);
    }

    #[test]
    fn test_materials_resized() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merger_material(BufferUpdateStatus::new_resized(true));

        assert_eq!(system_under_test.any_resized(), true);
        assert_eq!(system_under_test.any_updated(), true);
        assert_eq!(system_under_test.geometry_updated(), false);
    }

    #[test]
    fn test_geometry_and_materials_resized() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merge_geometry(BufferUpdateStatus::new_resized( true));
        system_under_test.merger_material(BufferUpdateStatus::new_resized(true));

        assert_eq!(system_under_test.any_resized(), true);
        assert_eq!(system_under_test.any_updated(), true);
        assert_eq!(system_under_test.geometry_updated(), true);
    }

    #[test]
    fn test_merge_multiple_statuses() {
        let mut system_under_test = BuffersUpdateStatus::new();

        system_under_test.merge_geometry(BufferUpdateStatus::new_resized(true));
        system_under_test.merge_geometry(BufferUpdateStatus::new_updated(true));
        system_under_test.merger_material(BufferUpdateStatus::new_resized(true));
        system_under_test.merger_material(BufferUpdateStatus::new_updated(true));

        assert_eq!(system_under_test.any_resized(), true);
        assert_eq!(system_under_test.any_updated(), true);
        assert_eq!(system_under_test.geometry_updated(), true);
    }
}