use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::utils::{create_frame_buffer_layer, frame_buffer_layer_size_bytes, FrameBufferLayerParameters, FrameBufferLayerParametersBuilder};
use bytemuck::{AnyBitPattern, Pod};
use futures_intrusive::channel::shared::oneshot_channel;
use std::marker::PhantomData;
use std::rc::Rc;
use wgpu::{BufferAddress, BufferUsages, CommandEncoder};

#[derive(PartialEq)]
pub(crate) enum SupportUpdateFromCpu {
    Yes,
    No,
}

pub(crate) struct FrameBufferLayer<T: Sized + AnyBitPattern + Pod> {
    gpu_located_render_target: Rc<wgpu::Buffer>,
    cpu_mappable_mediator: wgpu::Buffer,
    buffer_size_bytes: BufferAddress,
    
    _marker: PhantomData<T>,
}

impl<T: Sized + AnyBitPattern + Pod> FrameBufferLayer<T> {
    const LABEL_GPU_LOCATED_RENDER_TARGET: &'static str = " render target";
    const LABEL_CPU_MAPPABLE_MEDIATOR: &'static str = " cpu mappable mediator";

    #[must_use]
    pub(crate) fn new(device: &wgpu::Device, frame_buffer_size: FrameBufferSize, cpu_updatable: SupportUpdateFromCpu, marker: &str) -> Self {
        let mut render_target_usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;
        if cpu_updatable == SupportUpdateFromCpu::Yes { render_target_usage |= BufferUsages::COPY_DST }
        let render_target_label = format!("{} {}", marker, Self::LABEL_GPU_LOCATED_RENDER_TARGET);
        let parameters_gpu_located_render_target = Self::parameters(frame_buffer_size, render_target_usage, render_target_label.as_str());
        let gpu_located_copy = create_frame_buffer_layer(device, &parameters_gpu_located_render_target);
        
        let mediator_usage = BufferUsages::MAP_READ | BufferUsages::COPY_DST;
        let mediator_label = format!("{} {}", marker, Self::LABEL_CPU_MAPPABLE_MEDIATOR);
        let parameters_cpu_mappable_mediator = Self::parameters(frame_buffer_size, mediator_usage, mediator_label.as_str());
        let cpu_mappable_mediator = create_frame_buffer_layer(device, &parameters_cpu_mappable_mediator);

        let buffer_size_bytes: BufferAddress = frame_buffer_layer_size_bytes(&parameters_cpu_mappable_mediator);
        debug_assert_eq!(buffer_size_bytes, frame_buffer_layer_size_bytes(&parameters_gpu_located_render_target));

        Self {
            gpu_located_render_target: Rc::new(gpu_located_copy),
            cpu_mappable_mediator,
            buffer_size_bytes,
            
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub(crate) fn gpu_render_target(&self) -> Rc<wgpu::Buffer> {
        self.gpu_located_render_target.clone()
    }

    const ZERO_SOURCE_OFFSET: BufferAddress = 0;
    const ZERO_DESTINATION_OFFSET: BufferAddress = 0;

    pub(crate) fn issue_copy_to_cpu_mediator(&self, encoder: &mut CommandEncoder) {
        encoder.copy_buffer_to_buffer(
            &self.gpu_located_render_target,
            Self::ZERO_SOURCE_OFFSET,
            &self.cpu_mappable_mediator,
            Self::ZERO_DESTINATION_OFFSET,
            self.buffer_size_bytes as BufferAddress,
        );
    }

    #[cfg(feature = "denoiser")]
    pub(crate) fn fill_render_target(&self, queue: &wgpu::Queue, data: &[T]) {
        assert!(size_of_val(data) <= self.buffer_size_bytes as usize);
        queue.write_buffer(&self.gpu_located_render_target, Self::ZERO_DESTINATION_OFFSET, bytemuck::cast_slice(data));
    }

    pub(crate) fn read_cpu_mediator<ConsumeData: FnOnce(&[T])>(&self, consume: ConsumeData) -> impl Future<Output = ()> {
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
                let object_ids: &[T] = bytemuck::cast_slice(&raw_data);
                consume(object_ids);
            }
            
            self.cpu_mappable_mediator.unmap();
        }
    }

    #[must_use]
    fn parameters(frame_buffer_size: FrameBufferSize, usage: BufferUsages, label: &str) -> FrameBufferLayerParameters<'_> {
        FrameBufferLayerParametersBuilder::new(usage)
            .label(label)
            .frame_buffer_size(frame_buffer_size)
            .bytes_per_channel(size_of::<T>() as u32)
            .channels_count(1)
            .build()
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

        let system_under_test = FrameBufferLayer::<u32>::new(context.device(), test_buffer_size(), SupportUpdateFromCpu::No, "test layer");

        let actual_gpu_original_usage = system_under_test.gpu_render_target().usage();
        let expected_gpu_original_usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;
        assert_eq!(actual_gpu_original_usage, expected_gpu_original_usage);
    }

    #[test]
    fn test_read_staging() {
        let context = create_headless_wgpu_context();
        let buffer_size = test_buffer_size();
        let system_under_test = FrameBufferLayer::<u32>::new(context.device(), buffer_size, SupportUpdateFromCpu::No, "test layer");
        let callback_spy_call_counter = Rc::new(RefCell::new(0_u32));

        let read_callback = system_under_test.read_cpu_mediator(|data| {
            assert_eq!(data.len(), buffer_size.area() as usize);
            *callback_spy_call_counter.borrow_mut() += 1;
        });
        let poll_status = context.wait(None);
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
        let system_under_test = FrameBufferLayer::<u32>::new(context.device(), buffer_size, SupportUpdateFromCpu::No, "test layer");

        let mut encoder = context.device().create_command_encoder(&CommandEncoderDescriptor { label: None });
        system_under_test.issue_copy_to_cpu_mediator(&mut encoder);
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
