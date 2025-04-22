use std::mem;
use crate::bvh::node::BvhNode;
use crate::objects::triangle::Triangle;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::GpuSerializationSize;

pub(crate) struct GpuReadyTriangles {
    triangles: GpuReadySerializationBuffer,
    bvh: GpuReadySerializationBuffer,
}

impl GpuReadyTriangles {
    #[must_use]
    pub(crate) fn extract_geometry(&mut self) -> GpuReadySerializationBuffer {
        let replacement = GpuReadySerializationBuffer::make_filled(0, <Triangle as GpuSerializationSize>::SERIALIZED_QUARTET_COUNT, -1.0_f32);
        let result = mem::replace(&mut self.triangles, replacement);
        result
    }
    #[must_use]
    pub(crate) fn extract_bvh(&mut self) -> GpuReadySerializationBuffer {
        let replacement = GpuReadySerializationBuffer::make_filled(0, <BvhNode as GpuSerializationSize>::SERIALIZED_QUARTET_COUNT, -1.0_f32);
        let result = mem::replace(&mut self.bvh, replacement);
        result
    }
    
    #[must_use]
    pub(crate) fn empty(&self) -> bool {
        self.triangles.is_empty()
    }

    #[must_use]
    pub(super) fn new(triangles: GpuReadySerializationBuffer, bvh: GpuReadySerializationBuffer) -> Self {
        Self { triangles, bvh }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_empty() {
        let triangles = GpuReadySerializationBuffer::make_filled(0, 1, 0.0);
        let bvh = GpuReadySerializationBuffer::make_filled(1, 1, 0.0);
        let system_under_test = GpuReadyTriangles::new(triangles, bvh);

        assert!(system_under_test.empty());
    }
}