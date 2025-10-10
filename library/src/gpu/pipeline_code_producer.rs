use std::rc::Rc;
use wgpu::ShaderModule;
use crate::gpu::pipeline_code::PipelineCode;
use crate::gpu::resources::Resources;

pub(super) struct PipelineCodeProducer {
    module: Rc<ShaderModule>,
    hash: u64,
}

impl PipelineCodeProducer {
    #[must_use]
    pub(super) fn new(resource: &Resources, source_code: String) -> Self {
        let module = resource.create_shader_module("ray tracer shader", source_code.as_str());
        let hash = seahash::hash(source_code.as_bytes());
        Self {
            module,
            hash,
        }
    }

    #[must_use]
    pub(super) fn make(&self, human_readable_uid: &str) -> PipelineCode {
        PipelineCode::new(self.module.clone(), self.hash, human_readable_uid.to_string())
    }
}