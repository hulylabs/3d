use crate::geometry::aabb::Aabb;

#[derive(Copy, Clone)]
pub(crate) struct SceneObjectProxy {
    host_container_index: usize,
    aabb: Aabb,
}

impl SceneObjectProxy {
    #[must_use]
    pub(crate) fn new(host_container_index: usize, aabb: Aabb) -> Self {
        Self { host_container_index, aabb }
    }

    #[must_use]
    pub(crate) fn host_container_index(&self) -> usize {
        self.host_container_index
    }

    #[must_use]
    pub(crate) fn aabb(&self) -> Aabb {
        self.aabb
    }
}