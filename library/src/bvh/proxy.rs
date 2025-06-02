use crate::geometry::aabb::Aabb;

#[derive(Copy, Clone)]
pub(crate) enum PrimitiveType {
    Null = 0,
    Sdf = 1,
    Triangle = 2,
}

#[derive(Copy, Clone)]
pub(crate) struct SceneObjectProxy {
    host_container_index: usize,
    primitive_type: PrimitiveType,
    aabb: Aabb,
}

impl SceneObjectProxy {
    #[must_use]
    pub(crate) fn new(host_container_index: usize, primitive_type: PrimitiveType, aabb: Aabb,) -> Self {
        Self { host_container_index, primitive_type, aabb }
    }

    #[must_use]
    pub(crate) fn host_container_index(&self) -> usize {
        self.host_container_index
    }
    
    

    #[must_use]
    pub(crate) fn aabb(&self) -> Aabb {
        self.aabb
    }

    #[must_use]
    pub(crate) fn primitive_type(&self) -> PrimitiveType {
        self.primitive_type
    }
}