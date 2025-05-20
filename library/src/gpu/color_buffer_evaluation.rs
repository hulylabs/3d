use std::cell::{Ref, RefCell};
use std::rc::Rc;
use crate::gpu::compute_pipeline::ComputePipeline;

pub(super) struct ColorBufferEvaluation {
    ray_tracing_pipeline: Rc<RefCell<ComputePipeline>>,
    frame_counter_increment: u32,
    frame_counter_default: u32,
}

impl ColorBufferEvaluation {
    #[must_use]
    pub(super) fn new_monte_carlo(pipeline: Rc<RefCell<ComputePipeline>>) -> Self {
        Self { ray_tracing_pipeline: pipeline, frame_counter_increment: 1, frame_counter_default: 0, }
    }
    #[must_use]
    pub(super) fn new_deterministic(pipeline: Rc<RefCell<ComputePipeline>>) -> Self {
        Self { ray_tracing_pipeline: pipeline, frame_counter_increment: 0, frame_counter_default: 1, }
    }
    #[must_use]
    pub(super) fn pipeline(&self) -> Ref<ComputePipeline> {
        self.ray_tracing_pipeline.borrow()
    }
    #[must_use]
    pub(super) fn frame_counter_increment(&self) -> u32 {
        self.frame_counter_increment
    }
    #[must_use]
    pub(super) fn frame_counter_default(&self) -> u32 {
        self.frame_counter_default
    }
}