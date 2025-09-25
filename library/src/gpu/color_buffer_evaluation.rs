use std::cell::{Ref, RefCell};
use std::rc::Rc;
use crate::gpu::compute_pipeline::ComputePipeline;

#[derive(PartialEq, Copy, Clone)]
pub(crate) enum RenderStrategyId {
    MonteCarlo,
    Deterministic,
}

pub(super) struct ColorBufferEvaluationStrategy {
    ray_tracing_pipeline: Rc<RefCell<ComputePipeline>>,
    frame_counter_increment: u32,
    frame_counter_default: u32,
    id: RenderStrategyId,
}

impl ColorBufferEvaluationStrategy {
    #[must_use]
    pub(super) fn new_monte_carlo(pipeline: Rc<RefCell<ComputePipeline>>) -> Self {
        Self { ray_tracing_pipeline: pipeline, frame_counter_increment: 1, frame_counter_default: 0, id: RenderStrategyId::MonteCarlo, }
    }
    #[must_use]
    pub(super) fn new_deterministic(pipeline: Rc<RefCell<ComputePipeline>>) -> Self {
        Self { ray_tracing_pipeline: pipeline, frame_counter_increment: 0, frame_counter_default: 1, id: RenderStrategyId::Deterministic, }
    }
    
    #[must_use]
    pub(super) fn pipeline(&self) -> Ref<'_, ComputePipeline> {
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
    #[must_use]
    pub fn id(&self) -> RenderStrategyId {
        self.id
    }
}