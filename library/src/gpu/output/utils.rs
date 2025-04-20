use crate::gpu::frame_buffer_size::FrameBufferSize;
use wgpu::{BufferAddress, BufferUsages};

pub(super) struct FrameBufferLayerParameters<'a> {
    label: Option<&'a str>,
    size: FrameBufferSize,
    bytes_per_channel: u32,
    channels_count: u32,
    usage: BufferUsages,
}

pub(super) struct FrameBufferLayerParametersBuilder<'a> {
    label: Option<&'a str>,
    frame_buffer_size: Option<FrameBufferSize>,
    bytes_per_channel: Option<u32>,
    channels_count: Option<u32>,
    usage: BufferUsages,
}

impl<'a> FrameBufferLayerParametersBuilder<'a> {
    #[must_use]
    pub(super) fn new(usage: BufferUsages) -> Self {
        Self {
            label: None,
            frame_buffer_size: None,
            bytes_per_channel: None,
            channels_count: None,
            usage,
        }
    }

    pub(super) fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub(super) fn frame_buffer_size(mut self, size: FrameBufferSize) -> Self {
        self.frame_buffer_size = Some(size);
        self
    }

    pub(super) fn bytes_per_channel(mut self, bytes: u32) -> Self {
        assert!(bytes > 0);
        self.bytes_per_channel = Some(bytes);
        self
    }

    pub(super) fn channels_count(mut self, count: u32) -> Self {
        assert!(count > 0);
        self.channels_count = Some(count);
        self
    }

    #[must_use]
    pub(super) fn build(self) -> FrameBufferLayerParameters<'a> {
        FrameBufferLayerParameters {
            label: self.label,
            size: self.frame_buffer_size.expect("frame buffer size is required"),
            bytes_per_channel: self.bytes_per_channel.expect("channel size in bytes is required"),
            channels_count: self.channels_count.expect("channels count is required"),
            usage: self.usage,
        }
    }
}

#[must_use]
pub(super) fn frame_buffer_layer_size_bytes(parameters: &FrameBufferLayerParameters) -> BufferAddress {
    (parameters.size.area() * parameters.channels_count * parameters.bytes_per_channel) as BufferAddress
}

#[must_use]
pub(super) fn create_frame_buffer_layer(device: &wgpu::Device, parameters: &FrameBufferLayerParameters) -> wgpu::Buffer {
    let size_bytes = frame_buffer_layer_size_bytes(parameters);

    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: parameters.label,
        usage: parameters.usage,
        size: size_bytes,
        mapped_at_creation: false,
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;

    #[must_use]
    fn test_usage() -> BufferUsages {
        BufferUsages::COPY_DST | BufferUsages::STORAGE
    }

    #[test]
    fn test_builder_set_all_fields() {
        const EXPECTED_LABEL: &'static str = "Layer A";
        let expected_size = FrameBufferSize::new(1920, 1080);
        let expected_channels_count = 4;
        let expected_bytes_per_channel = 2;

        let actual_parameters = FrameBufferLayerParametersBuilder::new(test_usage())
            .label(EXPECTED_LABEL)
            .frame_buffer_size(expected_size)
            .bytes_per_channel(expected_bytes_per_channel)
            .channels_count(expected_channels_count)
            .build();

        assert_eq!(actual_parameters.label, Some(EXPECTED_LABEL));
        assert_eq!(actual_parameters.size, expected_size);
        assert_eq!(actual_parameters.bytes_per_channel, expected_bytes_per_channel);
        assert_eq!(actual_parameters.channels_count, expected_channels_count);
        assert_eq!(actual_parameters.usage, test_usage());
    }

    #[test]
    #[should_panic(expected = "channel size in bytes is required")]
    fn test_builder_missing_bytes_per_channel() {
        _ = FrameBufferLayerParametersBuilder::new(test_usage())
            .frame_buffer_size(FrameBufferSize::new(1280, 720))
            .channels_count(3)
            .build();
    }

    #[test]
    #[should_panic(expected = "channels count is required")]
    fn test_builder_missing_channels_count() {
        _ = FrameBufferLayerParametersBuilder::new(test_usage())
            .frame_buffer_size(FrameBufferSize::new(1280, 720))
            .bytes_per_channel(1)
            .build();
    }

    #[test]
    #[should_panic(expected = "frame buffer size is required")]
    fn test_builder_missing_size() {
        _ = FrameBufferLayerParametersBuilder::new(test_usage()).bytes_per_channel(1).channels_count(3).build();
    }

    #[test]
    fn test_frame_buffer_layer_size_bytes() {
        let size = FrameBufferSize::new(10, 10);
        let channel_count: u32 = 4;
        let bytes_per_channel: u32 = 3;

        let parameters = FrameBufferLayerParametersBuilder::new(test_usage())
            .frame_buffer_size(size)
            .bytes_per_channel(bytes_per_channel)
            .channels_count(channel_count)
            .build();

        let total_bytes = frame_buffer_layer_size_bytes(&parameters);
        assert_eq!(total_bytes, (size.area() * channel_count * bytes_per_channel) as BufferAddress);
    }

    #[test]
    fn test_create_frame_buffer_layer_does_not_panic() {
        let parameters = FrameBufferLayerParametersBuilder::new(test_usage())
            .frame_buffer_size(FrameBufferSize::new(32, 64))
            .bytes_per_channel(4)
            .channels_count(3)
            .build();
        let context = create_headless_wgpu_context();

        let actual_layer = create_frame_buffer_layer(context.device(), &parameters);

        assert_eq!(actual_layer.size(), frame_buffer_layer_size_bytes(&parameters));
        assert_eq!(actual_layer.usage(), test_usage());
    }
}
