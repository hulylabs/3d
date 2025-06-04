use log::info;
use wgpu::Adapter;

pub(crate) struct AdapterFeatures {
    desired_features: wgpu::Features,
    pipeline_caching_supported: bool,
}

impl AdapterFeatures {
    #[must_use]
    pub(crate) fn new(adapter: &Adapter) -> Self {
        let adapter_features = adapter.features();
        
        if adapter_features.contains(wgpu::Features::PIPELINE_CACHE) {
            AdapterFeatures { desired_features: wgpu::Features::PIPELINE_CACHE, pipeline_caching_supported: true, }
        } else {
            AdapterFeatures { desired_features: wgpu::Features::empty(), pipeline_caching_supported: false, }
        }
    }

    #[must_use]
    pub(crate) fn desired_features(&self) -> wgpu::Features {
        self.desired_features
    }

    #[must_use]
    pub(crate) fn pipeline_caching_supported(&self) -> bool {
        self.pipeline_caching_supported
    }
}

pub(crate) fn log_adapter_info(adapter: &Adapter) {
    let adapter_info = adapter.get_info();
    info!(
        "Adapter Info:\n\
         Name: {}\n\
         Backend: {:?}\n\
         Vendor: {:#x}\n\
         Device: {:#x}\n\
         Device Type: {:?}\n\
         Driver: {:?}\n\
         Driver Info: {:?}",
        adapter_info.name,
        adapter_info.backend,
        adapter_info.vendor,
        adapter_info.device,
        adapter_info.device_type,
        adapter_info.driver,
        adapter_info.driver_info,
    );
}
