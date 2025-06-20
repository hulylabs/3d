use crate::container::scene_object::{SceneEnvironment, SceneObject};
use crate::geometry::transform::Affine;
use crate::objects::material_index::MaterialIndex;
use crate::objects::ray_traceable::RayTraceable;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(super) struct Monolithic {
    geometry_kind: usize,
    geometry: Box<dyn RayTraceable>,
    payload: usize,
    transformation: Affine,
}

impl Monolithic {
    #[must_use]
    pub(super) fn new(geometry_kind: usize, backend: Box<dyn RayTraceable>, payload: usize, transformation: Affine) -> Self {
        Self {
            geometry_kind,
            geometry: backend,
            payload,
            transformation,
        }
    }
}

impl SceneObject for Monolithic {
    #[must_use]
    fn material(&self) -> MaterialIndex {
        self.geometry.material()
    }
    fn set_material(&mut self, new_material: MaterialIndex, _environment: &mut SceneEnvironment) {
        self.geometry.set_material(new_material)
    }

    #[must_use]
    fn data_kind_uid(&self) -> usize {
        self.geometry_kind
    }
    #[must_use]
    fn payload(&self) -> usize {
        self.payload
    }
    #[must_use]
    fn transformation(&self) -> &Affine {
        &self.transformation
    }

    #[must_use]
    fn serialized_quartet_count(&self) -> usize {
        self.geometry.serialized_quartet_count()
    }
    fn serialize_into(&self, buffer: &mut GpuReadySerializationBuffer) {
        self.geometry.serialize_into(buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::serializable_for_gpu::GpuSerializable;
    use cgmath::Deg;

    struct DummyRayTraceable;

    impl GpuSerializable for DummyRayTraceable {
        fn serialize_into(&self, _buffer: &mut GpuReadySerializationBuffer) {}
    }

    impl RayTraceable for DummyRayTraceable {
        #[must_use]
        fn material(&self) -> MaterialIndex {
            MaterialIndex(0)
        }

        fn set_material(&mut self, _material_index: MaterialIndex) {}

        #[must_use]
        fn serialized_quartet_count(&self) -> usize {
            0
        }
    }

    #[test]
    fn test_payload_pass_through() {
        let expected_geometry_kind = 17;
        let expected_payload = 3;
        let system_under_test = Monolithic::new(expected_geometry_kind, Box::new(DummyRayTraceable {}), expected_payload, Affine::from_angle_y(Deg(45.0)));

        assert_eq!(system_under_test.payload(), expected_payload);
        assert_eq!(system_under_test.data_kind_uid(), expected_geometry_kind);
    }
}
