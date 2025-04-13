use std::collections::HashMap;
use wgpu::{BindGroup, ComputePass, RenderPass};
use crate::gpu::bind_group_builder::BindGroupBuilder;

pub(super) struct BindingGroups {
    slots_distribution: HashMap<u32, BindGroup>
}

impl BindingGroups {
    #[must_use]
    pub(super) fn new() -> Self {
        Self { slots_distribution: HashMap::new() }
    }

    pub(super) fn insert(&mut self, device: &wgpu::Device, bind_group_builder: BindGroupBuilder) {
        let bind_group = bind_group_builder.make_bind_group(device);
        self.slots_distribution.insert(bind_group_builder.index(), bind_group);
    }

    pub(super) fn set_into_compute_pass(&self, pass: &mut ComputePass) {
        self.slots_distribution.iter().for_each(|(index, bind_group)| {
            pass.set_bind_group(*index, bind_group, &[]);
        })
    }

    pub(super) fn set_into_render_pass(&self, pass: &mut RenderPass) {
        self.slots_distribution.iter().for_each(|(index, bind_group)| {
            pass.set_bind_group(*index, bind_group, &[]);
        })
    }
}