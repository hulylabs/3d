use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::utils::{FrameBufferLayerParameters, FrameBufferLayerParametersBuilder, create_frame_buffer_layer, frame_buffer_layer_size_bytes};
use futures_intrusive::channel::shared::oneshot_channel;
use std::rc::Rc;
use wgpu::{BufferAddress, BufferUsages, CommandEncoder};

pub(super) struct ObjectIdLayer {
    gpu_located_render_target: Rc<wgpu::Buffer>,
    cpu_mappable_mediator: wgpu::Buffer,
    buffer_size_bytes: BufferAddress,
}

type ObjectId = u32;

impl ObjectIdLayer {
    const CHANNELS_COUNT: u32 = 1;

    const LABEL_GPU_LOCATED_RENDER_TARGET: &'static str = "object id render target";
    const LABEL_CPU_MAPPABLE_MEDIATOR: &'static str = "object id cpu mappable mediator";

    #[must_use]
    pub(super) fn new(device: &wgpu::Device, frame_buffer_size: FrameBufferSize) -> Self {
        fn parameters(frame_buffer_size: FrameBufferSize, usage: BufferUsages, label: &str) -> FrameBufferLayerParameters {
            FrameBufferLayerParametersBuilder::new(usage)
                .label(label)
                .frame_buffer_size(frame_buffer_size)
                .bytes_per_channel(size_of::<ObjectId>() as u32)
                .channels_count(ObjectIdLayer::CHANNELS_COUNT)
                .build()
        }

        let parameters_gpu_located_render_target = parameters(frame_buffer_size, BufferUsages::STORAGE | BufferUsages::COPY_SRC, Self::LABEL_GPU_LOCATED_RENDER_TARGET);
        let gpu_located_copy = create_frame_buffer_layer(device, &parameters_gpu_located_render_target);

        let parameters_cpu_mappable_mediator = parameters(frame_buffer_size, BufferUsages::MAP_READ | BufferUsages::COPY_DST, Self::LABEL_CPU_MAPPABLE_MEDIATOR);
        let cpu_mappable_copy = create_frame_buffer_layer(device, &parameters_cpu_mappable_mediator);

        let buffer_size_bytes: BufferAddress = frame_buffer_layer_size_bytes(&parameters_cpu_mappable_mediator);
        debug_assert_eq!(buffer_size_bytes, frame_buffer_layer_size_bytes(&parameters_gpu_located_render_target));

        ObjectIdLayer {
            gpu_located_render_target: Rc::new(gpu_located_copy),
            cpu_mappable_mediator: cpu_mappable_copy,
            buffer_size_bytes,
        }
    }

    #[must_use]
    pub fn gpu_render_target(&self) -> Rc<wgpu::Buffer> {
        self.gpu_located_render_target.clone()
    }

    pub(super) fn issue_copy_to_staging(&self, encoder: &mut CommandEncoder) {
        const SOURCE_OFFSET: BufferAddress = 0;
        const DESTINATION_OFFSET: BufferAddress = 0;

        encoder.copy_buffer_to_buffer(
            &self.gpu_located_render_target,
            SOURCE_OFFSET,
            &self.cpu_mappable_mediator,
            DESTINATION_OFFSET,
            self.buffer_size_bytes as BufferAddress,
        );
    }

    pub(crate) fn read_staging<ConsumeData: FnOnce(&[ObjectId])>(&self, consume: ConsumeData) -> impl Future<Output = ()> {
        let cpu_mediator_slice = self.cpu_mappable_mediator.slice(..);

        let (sender, receiver) = oneshot_channel();
        cpu_mediator_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).expect("cpu_mediator_slice.map_async executed the callback, but result send failed");
        });

        async move {
            let map_result = receiver.receive().await;
            map_result.expect("the result of 'map' operation is unknown").expect("'map' operation has failed");
            {
                let raw_data = cpu_mediator_slice.get_mapped_range();
                let object_ids: &[ObjectId] = bytemuck::cast_slice(&raw_data);
                consume(object_ids);
            }

            self.cpu_mappable_mediator.unmap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use futures_intrusive::channel::shared::oneshot_channel;
    use std::cell::RefCell;
    use wgpu::wgt::PollType;
    use wgpu::{CommandEncoderDescriptor, PollStatus};

    #[must_use]
    fn test_buffer_size() -> FrameBufferSize {
        FrameBufferSize::new(1024, 768)
    }

    #[test]
    fn test_construction() {
        let context = create_headless_wgpu_context();

        let system_under_test = ObjectIdLayer::new(context.device(), test_buffer_size());

        let actual_gpu_original_usage = system_under_test.gpu_render_target().usage();
        let expected_gpu_original_usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;
        assert_eq!(actual_gpu_original_usage, expected_gpu_original_usage);
    }

    #[test]
    fn test_read_staging() {
        let context = create_headless_wgpu_context();
        let buffer_size = test_buffer_size();
        let system_under_test = ObjectIdLayer::new(context.device(), buffer_size);
        let callback_spy_call_counter = Rc::new(RefCell::new(0_u32));

        let read_callback = system_under_test.read_staging(|data| {
            assert_eq!(data.len(), buffer_size.area() as usize);
            *callback_spy_call_counter.borrow_mut() += 1;
        });
        let poll_status = context.device().poll(PollType::Wait).expect("failed to poll the device");
        assert_eq!(poll_status, PollStatus::QueueEmpty);
        pollster::block_on(read_callback);

        assert_eq!(
            *callback_spy_call_counter.borrow(),
            1,
            "callback is expected to be called once, but called {} times",
            *callback_spy_call_counter.borrow()
        );
    }

    #[test]
    fn test_issue_copy_to_staging() {
        let context = create_headless_wgpu_context();
        let buffer_size = test_buffer_size();
        let system_under_test = ObjectIdLayer::new(context.device(), buffer_size);

        let mut encoder = context.device().create_command_encoder(&CommandEncoderDescriptor { label: None });
        system_under_test.issue_copy_to_staging(&mut encoder);
        context.queue().submit(Some(encoder.finish()));

        let (copy_request_finished_cast, copy_request_finished_signal) = oneshot_channel();
        context.queue().on_submitted_work_done(Box::new(move || {
            copy_request_finished_cast.send(()).unwrap();
        }));

        let poll_status = context.device().poll(PollType::Wait).expect("failed to poll the device");
        assert_eq!(poll_status, PollStatus::QueueEmpty);
        pollster::block_on(copy_request_finished_signal.receive());
    }
}
