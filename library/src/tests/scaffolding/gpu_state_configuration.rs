#[cfg(test)]
pub(crate) mod tests {
    use std::time::Instant;
    use cgmath::EuclideanSpace;
    use crate::bvh::builder::build_serialized_bvh;
    use crate::container::bvh_proxies::proxy_of_sdf;
    use crate::container::sdf_warehouse::SdfWarehouse;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::transform::Affine;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::pipelines_factory::ComputeRoutineEntryPoint;
    use crate::gpu::uniforms::Uniforms;
    use crate::material::material_index::MaterialIndex;
    use crate::objects::common_properties::Linkage;
    use crate::objects::sdf_class_index::SdfClassIndex;
    use crate::objects::sdf_instance::SdfInstance;
    use crate::scene::camera::Camera;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::framework::sdf_registrator::SdfRegistrator;
    use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
    use crate::serialization::pod_vector::PodVector;
    use crate::serialization::serializable_for_gpu::{GpuSerializable, GpuSerializationSize};
    use crate::tests::scaffolding::dummy_implementations::tests::TEST_DATA_IO_BINDING_GROUP;
    use crate::tests::scaffolding::gpu_code_execution::tests::{DataBindGroupSlot, ExecutionConfig};
    use crate::utils::object_uid::ObjectUid;

    impl ExecutionConfig {
        pub(crate) fn common_test_config(&mut self) -> &mut Self {
            self
                .set_test_data_binding_group(TEST_DATA_IO_BINDING_GROUP)
                .set_entry_point(ComputeRoutineEntryPoint::TestDefault)
        }
    }

    #[must_use]
    pub(crate) fn config_empty_bindings() -> ExecutionConfig {
        let mut ware = ExecutionConfig::new();
        ware
            .common_test_config()
            .set_dummy_binding_group(0, vec![], vec![], vec![])
            .set_dummy_binding_group(1, vec![], vec![], vec![])
            .set_dummy_binding_group(2, vec![], vec![], vec![])
        ;
        ware
    }

    #[must_use]
    pub(crate) fn config_common_sdf_buffers() -> ExecutionConfig {
        let mut uniforms = make_test_uniforms();
        uniforms.set_bvh_length(1);
        uniforms.set_parallelograms_count(0);

        let mut ware = ExecutionConfig::new();
        ware
            .common_test_config()
            .set_dummy_binding_group(1, vec![], vec![], vec![])
            .set_storage_binding_group(0, vec![], vec![
                DataBindGroupSlot::new(0, uniforms.serialize().backend()),
            ])
        ;
        ware
    }

    #[must_use]
    pub(crate) fn make_test_uniforms() -> Uniforms {
        let dummy_camera = Camera::new_orthographic_camera(1.0, Point::origin());
        let dummy_frame_buffer_size = FrameBufferSize::new(1, 1);
        Uniforms::new(dummy_frame_buffer_size, dummy_camera, 1, Instant::now().elapsed())
    }

    #[must_use]
    pub(crate) fn config_sdf_sampling(serialized_sdf: SdfInstances) -> ExecutionConfig {
        let mut execution_config = config_common_sdf_buffers();
        execution_config.set_storage_binding_group(2, vec![], vec![
            DataBindGroupSlot::new(1, serialized_sdf.instances.backend()),
            DataBindGroupSlot::new(5, serialized_sdf.inflated_bvh.backend()),
            DataBindGroupSlot::new(6, bytemuck::bytes_of(&[0_f32; 1])),
        ]);
        execution_config
    }

    #[must_use]
    pub(crate) fn config_sdf_shadow_sampling(serialized_sdf: SdfInstances) -> ExecutionConfig {
        let mut execution_config = config_common_sdf_buffers();
        let dummy_buffer = [0_u8; 96];
        execution_config.set_storage_binding_group(2, vec![], vec![
            DataBindGroupSlot::new(0, &dummy_buffer),
            DataBindGroupSlot::new(1, serialized_sdf.instances.backend()),
            DataBindGroupSlot::new(2, &dummy_buffer),
            DataBindGroupSlot::new(3, &dummy_buffer),
            DataBindGroupSlot::new(4, serialized_sdf.bvh.backend()),
            DataBindGroupSlot::new(5, serialized_sdf.inflated_bvh.backend()),
            DataBindGroupSlot::new(6, bytemuck::bytes_of(&[0_f32; 1])),
        ]);
        execution_config
    }

    pub(crate) struct SdfInstances {
        instances: GpuReadySerializationBuffer,
        bvh: GpuReadySerializationBuffer,
        inflated_bvh: GpuReadySerializationBuffer,
    }

    impl SdfInstances {
        #[must_use]
        pub(crate) fn instances(&self) -> &GpuReadySerializationBuffer {
            &self.instances
        }
    }

    #[must_use]
    pub(crate) fn make_single_serialized_sdf_instance(class: &NamedSdf, instance_transformation: &Affine) -> SdfInstances {
        let dummy_linkage = Linkage::new(ObjectUid(0), MaterialIndex(0));

        let sdf_instance = SdfInstance::new(instance_transformation.clone(), 1.0, SdfClassIndex(0), dummy_linkage);
        let mut instances = GpuReadySerializationBuffer::new(1, SdfInstance::SERIALIZED_QUARTET_COUNT);
        sdf_instance.serialize_into(&mut instances);

        #[must_use]
        fn make_bvh(sdf: &NamedSdf, instance_transformation: &Affine, inflation: f64) -> GpuReadySerializationBuffer {
            let aabb = sdf.sdf().aabb().transform(&instance_transformation).extent_relative_inflate(inflation);
            let mut support = [proxy_of_sdf(0, aabb)];
            build_serialized_bvh(&mut support)
        }

        let inflated_bvh = make_bvh(class, instance_transformation, 0.1);
        let bvh = make_bvh(class, instance_transformation, 0.0);

        SdfInstances { instances, bvh, inflated_bvh }
    }

    #[must_use]
    pub(crate) fn generate_code_for(sdf: &NamedSdf) -> String {
        let mut registrator = SdfRegistrator::new();
        registrator.add(&sdf);

        let warehouse = SdfWarehouse::new(registrator);
        warehouse.sdf_classes_code().to_string()
    }

    #[must_use]
    pub(crate) fn make_dummy_sdf_name() -> UniqueSdfClassName {
        UniqueSdfClassName::new("some_sdf".to_string())
    }

    #[must_use]
    pub(crate) fn to_pod(from: Vector) -> PodVector {
        PodVector::new(from.x as f32, from.y as f32, from.z as f32)
    }
}