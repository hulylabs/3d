use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::scene::camera::Camera;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use std::time::Duration;
use winit::dpi::PhysicalSize;

pub(crate) struct Uniforms {
    frame_buffer_size: FrameBufferSize,
    frame_number: u32,
    camera: Camera,
    
    parallelograms_count: u32,
    bvh_length: u32,
    pixel_side_subdivision: u32,

    global_time_seconds: f32,
}

impl Uniforms {
    #[must_use]
    pub(crate) fn new(frame_buffer_size: FrameBufferSize, camera: Camera, pixel_side_subdivision: u32, current_time: Duration) -> Self {
        Self {
            frame_buffer_size,
            frame_number: 0,
            camera,
            parallelograms_count: 0,
            bvh_length: 0,
            pixel_side_subdivision,
            global_time_seconds: current_time.as_secs_f32(),
        }
    }
    
    pub(super) fn reset_frame_accumulation(&mut self, value: u32) {
        self.frame_number = value;
    }

    pub(super) fn set_frame_size(&mut self, new_size: PhysicalSize<u32>) {
        self.frame_buffer_size = FrameBufferSize::new(new_size.width, new_size.height);
    }

    pub(super) fn next_frame(&mut self, increment: u32) {
        self.frame_number += increment;
    }

    pub(super) fn update_time(&mut self, current_time: Duration) {
        self.global_time_seconds = current_time.as_secs_f32();
    }

    #[must_use]
    pub(super) fn frame_buffer_area(&self) -> u32 {
        self.frame_buffer_size.area()
    }

    pub(super) fn set_pixel_side_subdivision(&mut self, level: u32) {
        let level: u32 = if 0 == level { 1 } else { level };
        self.pixel_side_subdivision = level;
    }

    pub(crate) fn set_parallelograms_count(&mut self, parallelograms_count: u32) {
        self.parallelograms_count = parallelograms_count;
    }

    pub(crate) fn set_bvh_length(&mut self, bvh_length: u32) {
        self.bvh_length = bvh_length;
    }

    #[must_use]
    pub(super) fn frame_buffer_size(&self) -> FrameBufferSize {
        self.frame_buffer_size
    }

    #[cfg(feature = "denoiser")]
    #[must_use]
    pub(super) fn frame_number(&self) -> u32 {
        self.frame_number
    }

    #[must_use]
    pub(super) fn mutable_camera(&mut self) -> &mut Camera {
        &mut self.camera
    }

    const SERIALIZED_QUARTET_COUNT: usize = 3 + Camera::SERIALIZED_QUARTET_COUNT;

    #[must_use]
    pub(crate) fn serialize(&self) -> GpuReadySerializationBuffer {
        let mut result = GpuReadySerializationBuffer::new(1, Self::SERIALIZED_QUARTET_COUNT);

        result.write_quartet(|writer| {
            writer.write_unsigned(self.frame_buffer_size.width());
            writer.write_unsigned(self.frame_buffer_size.height());
            writer.write_unsigned(self.frame_buffer_size.area());
            writer.write_float_32(self.frame_buffer_size.aspect());
        });
        
        result.write_quartet_f32(
           1.0 / self.frame_buffer_size.width() as f32,
           1.0 / self.frame_buffer_size.height() as f32,
           self.frame_number as f32,
           0.0,
        );
        
        self.camera.serialize_into(&mut result);

        result.write_quartet(|writer| {
            writer.write_unsigned(self.parallelograms_count);
            writer.write_unsigned(self.bvh_length);
            writer.write_unsigned(self.pixel_side_subdivision);
            writer.write_float_32(self.global_time_seconds);
        });
        
        debug_assert!(result.object_fully_written());
        result
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use super::*;
    use crate::geometry::alias::Point;
    use cgmath::EuclideanSpace;

    const DEFAULT_FRAME_WIDTH: u32 = 800;
    const DEFAULT_FRAME_HEIGHT: u32 = 600;

    const DEFAULT_PARALLELOGRAMS_COUNT: u32 = 5;
    const DEFAULT_BVH_LENGTH: u32 = 8;
    const DEFAULT_PIXEL_SIDE_SUBDIVISION: u32 = 4;
    const DEFAULT_GLOBAL_TIME_SECONDS: f32 = 5.0;

    const SLOT_FRAME_WIDTH: usize = 0;
    const SLOT_FRAME_HEIGHT: usize = 1;
    const SLOT_FRAME_AREA: usize = 2;
    const SLOT_FRAME_ASPECT: usize = 3;

    const SLOT_FRAME_INVERTED_WIDTH: usize = 4;
    const SLOT_FRAME_INVERTED_HEIGHT: usize = 5;
    const SLOT_FRAME_NUMBER: usize = 6;

    const SLOT_PARALLELOGRAMS_COUNT: usize = 40;
    const SLOT_BVH_LENGTH: usize = 41;
    const SLOT_PIXEL_SIDE_SUBDIVISION: usize = 42;
    const SLOT_GLOBAL_TIME: usize = 43;

    #[must_use]
    pub(crate) fn make_test_uniforms_instance() -> Uniforms {
        let frame_buffer_size = FrameBufferSize::new(DEFAULT_FRAME_WIDTH, DEFAULT_FRAME_HEIGHT);
        let camera = Camera::new_perspective_camera(1.0, Point::origin());

        Uniforms {
            frame_buffer_size,
            frame_number: 0,
            camera,

            parallelograms_count: DEFAULT_PARALLELOGRAMS_COUNT,
            bvh_length: DEFAULT_BVH_LENGTH,
            pixel_side_subdivision: DEFAULT_PIXEL_SIDE_SUBDIVISION,
            global_time_seconds: DEFAULT_GLOBAL_TIME_SECONDS,
        }
    }

    #[test]
    fn test_uniforms_reset_frame_accumulation() {
        let mut system_under_test = make_test_uniforms_instance();

        system_under_test.next_frame(1);
        system_under_test.reset_frame_accumulation(0);

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 0.0);
    }

    #[test]
    fn test_uniforms_set_frame_size() {
        let expected_width = 1024;
        let expected_height = 768;
        let new_size = PhysicalSize::new(expected_width, expected_height);
        let mut system_under_test = make_test_uniforms_instance();

        system_under_test.set_frame_size(new_size);

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_WIDTH].to_bits(), expected_width);
        assert_eq!(actual_state_floats[SLOT_FRAME_HEIGHT].to_bits(), expected_height);
        assert_eq!(actual_state_floats[SLOT_FRAME_AREA].to_bits(), expected_width * expected_height);
        assert_eq!(actual_state_floats[SLOT_FRAME_ASPECT], expected_width as f32 / expected_height as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_INVERTED_WIDTH], 1.0 / expected_width as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_INVERTED_HEIGHT], 1.0 / expected_height as f32);
    }

    #[test]
    fn test_uniforms_next_frame() {
        let mut system_under_test = make_test_uniforms_instance();

        system_under_test.next_frame(1);
        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 1.0);

        system_under_test.next_frame(1);
        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 2.0);
    }

    #[test]
    fn test_uniforms_frame_buffer_area() {
        let system_under_test = make_test_uniforms_instance();

        let expected_area = DEFAULT_FRAME_WIDTH * DEFAULT_FRAME_HEIGHT;
        assert_eq!(system_under_test.frame_buffer_area(), expected_area);
    }

    #[test]
    fn test_uniforms_serialize() {
        let mut system_under_test = make_test_uniforms_instance();

        let expected_time = Instant::now().elapsed();
        system_under_test.update_time(expected_time);

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_WIDTH].to_bits(), DEFAULT_FRAME_WIDTH);
        assert_eq!(actual_state_floats[SLOT_FRAME_HEIGHT].to_bits(), DEFAULT_FRAME_HEIGHT);
        assert_eq!(actual_state_floats[SLOT_FRAME_AREA].to_bits(), DEFAULT_FRAME_WIDTH * DEFAULT_FRAME_HEIGHT);
        assert_eq!(actual_state_floats[SLOT_FRAME_ASPECT], DEFAULT_FRAME_WIDTH as f32 / DEFAULT_FRAME_HEIGHT as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_INVERTED_WIDTH], 1.0 / DEFAULT_FRAME_WIDTH as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_INVERTED_HEIGHT], 1.0 / DEFAULT_FRAME_HEIGHT as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 0.0);

        assert_eq!(actual_state_floats[SLOT_PARALLELOGRAMS_COUNT].to_bits(), DEFAULT_PARALLELOGRAMS_COUNT);
        assert_eq!(actual_state_floats[SLOT_BVH_LENGTH].to_bits(), DEFAULT_BVH_LENGTH);
        assert_eq!(actual_state_floats[SLOT_PIXEL_SIDE_SUBDIVISION].to_bits(), DEFAULT_PIXEL_SIDE_SUBDIVISION);
        assert_eq!(actual_state_floats[SLOT_GLOBAL_TIME], expected_time.as_secs_f32());
    }
}