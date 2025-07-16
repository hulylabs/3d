use crate::geometry::epsilon::DEFAULT_EPSILON_F32;
use bytemuck::{Pod, Zeroable};
use cgmath::AbsDiffEq;
use std::fmt::{Display, Formatter, };

#[repr(C)]
#[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
pub(crate) struct PodVector {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) w: f32,
}

impl PodVector {
    #[must_use] #[cfg(test)]
    pub(crate) fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z, w: 0.0 }
    }
    #[must_use] #[cfg(test)]
    pub(crate) fn new_full(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w, }
    }
}

impl Display for PodVector {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

impl AbsDiffEq for PodVector {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        DEFAULT_EPSILON_F32
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.x.abs_diff_eq(&other.x, epsilon) && 
        self.y.abs_diff_eq(&other.y, epsilon) && 
        self.z.abs_diff_eq(&other.z, epsilon) && 
        self.w.abs_diff_eq(&other.w, epsilon)
    }
}
