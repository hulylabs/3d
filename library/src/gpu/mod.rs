pub(super) mod color_buffer_evaluation;

pub(crate) mod resources;
pub(crate) mod headless_device;
pub(crate) mod render;
pub(crate) mod frame_buffer_size;
pub(crate) mod context;
pub(crate) mod output;
pub(crate) mod compute_pipeline;
pub(crate) mod bind_group_builder;

mod binding_groups;
mod rasterization_pipeline;
mod versioned_buffer;
mod buffers_update_status;
pub(crate) mod pipelines_factory;
pub(crate) mod pipeline_code;
pub(crate) mod adapter_features;
mod resizable_buffer;
pub(crate) mod uniforms;
mod bitmap_textures;