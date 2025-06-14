﻿use wgpu::{AdapterInfo, PollStatus};
use wgpu::wgt::PollType;

pub(crate) struct Context {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline_caching_supported: bool,
    adapter_info: AdapterInfo,
}

impl Context {
    #[must_use]
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue, pipeline_caching_supported: bool, adapter_info: AdapterInfo,) -> Self {
        Self { device, queue, pipeline_caching_supported, adapter_info, }
    }

    #[must_use]
    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[must_use]
    pub(crate) fn pipeline_caching_supported(&self) -> bool {
        self.pipeline_caching_supported
    }

    #[must_use]
    pub(super) fn adapter_info(&self) -> &AdapterInfo {
        &self.adapter_info
    }

    pub(crate) fn wait(&self) -> PollStatus {
        self.device.poll(PollType::Wait).expect("failed to poll the device")
    }
}
