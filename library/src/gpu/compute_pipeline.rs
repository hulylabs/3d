use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::binding_groups::BindingGroups;
use wgpu::ComputePass;

pub(crate) struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    binding_groups: BindingGroups,
}

impl ComputePipeline {
    #[must_use]
    pub(crate) fn new(pipeline: wgpu::ComputePipeline) -> Self {
        Self { pipeline, binding_groups: BindingGroups::new() }
    }

    pub(crate) fn setup_bind_group<GroupSetup>(&mut self, index: u32, label: Option<&str>, device: &wgpu::Device, setup_code: GroupSetup)
        where
        GroupSetup: FnOnce(&mut BindGroupBuilder)
    {
        let layout = self.pipeline.get_bind_group_layout(index);
        let mut bind_group_builder = BindGroupBuilder::new(index, label, layout);

        setup_code(&mut bind_group_builder);

        self.binding_groups.insert(device, bind_group_builder);
    }

    pub(crate) fn set_into_pass(&self, pass: &mut ComputePass) {
        pass.set_pipeline(&self.pipeline);
        self.binding_groups.set_into_compute_pass(pass);
    }
}
