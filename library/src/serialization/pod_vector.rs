use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(PartialEq, Copy, Clone, Pod, Debug, Default, Zeroable)]
pub(crate) struct PodVector {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
    pub(crate) w: f32,
}