use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::binding_groups::BindingGroups;
use wgpu::{BindGroupLayout, ComputePass};

pub(super) struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    binding_groups: BindingGroups,
}

impl ComputePipeline {
    #[must_use]
    pub(super) fn new(pipeline: wgpu::ComputePipeline) -> Self {
        Self { pipeline, binding_groups: BindingGroups::new() }
    }

    #[must_use]
    pub(super) fn bind_group_layout(&mut self, index: u32) -> BindGroupLayout {
        self.pipeline.get_bind_group_layout(index)
    }

    pub(super) fn commit_bind_group(&mut self, device: &wgpu::Device, bind_group_builder: BindGroupBuilder) {
        self.binding_groups.insert(device, bind_group_builder);
    }

    pub(super) fn set_into_pass(&self, pass: &mut ComputePass) {
        pass.set_pipeline(&self.pipeline);
        self.binding_groups.set_into_compute_pass(pass);
    }
}
