﻿use crate::animation::time_tracker::TimeTracker;
use crate::bvh::node::BvhNode;
use crate::container::visual_objects::{DataKind, VisualObjects};
use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::buffers_update_status::BuffersUpdateStatus;
use crate::gpu::color_buffer_evaluation::{ColorBufferEvaluationStrategy, RenderStrategyId};
use crate::gpu::compute_pipeline::ComputePipeline;
use crate::gpu::context::Context;
use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::frame_buffer::FrameBuffer;
use crate::gpu::output::frame_buffer_layer::{FrameBufferLayer, SupportUpdateFromCpu};
use crate::gpu::pipeline_code::PipelineCode;
use crate::gpu::pipelines_factory::{ComputeRoutineEntryPoint, PipelinesFactory};
use crate::gpu::rasterization_pipeline::RasterizationPipeline;
use crate::gpu::resizable_buffer::ResizableBuffer;
use crate::gpu::resources::Resources;
use crate::gpu::uniforms::Uniforms;
use crate::gpu::versioned_buffer::{BufferUpdateStatus, VersionedBuffer};
use crate::objects::material::Material;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf_instance::SdfInstance;
use crate::objects::triangle::Triangle;
use crate::scene::camera::Camera;
use crate::scene::hub::Hub;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::pod_vector::PodVector;
use crate::serialization::serializable_for_gpu::GpuSerializationSize;
use crate::utils::object_uid::ObjectUid;
use cgmath::Vector2;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::rc::Rc;
use wgpu::{BufferAddress, CommandEncoder, StoreOp, SubmissionIndex};
use winit::dpi::PhysicalSize;

#[cfg(feature = "denoiser")]
mod denoiser {
    pub(super) use crate::denoiser::entry::Denoiser;
    pub(super) use crate::utils::min_max_time_measurer::MinMaxTimeMeasurer;
    pub(super) use exr::prelude::write_rgba_file;
    pub(super) use pxm::PFMBuilder;
    pub(super) use std::fs::File;
    pub(super) use std::path::Path;
}

pub(crate) struct Renderer {
    gpu: Gpu,
    uniforms: Uniforms,
    pipeline_ray_tracing_monte_carlo: Rc<RefCell<ComputePipeline>>,
    pipeline_ray_tracing_deterministic: Rc<RefCell<ComputePipeline>>,
    color_buffer_evaluation: ColorBufferEvaluationStrategy,
    pipeline_surface_attributes: ComputePipeline,
    pipeline_final_image_rasterization: RasterizationPipeline,
    scene: Hub,
    
    #[cfg(feature = "denoiser")]
    denoiser: denoiser::Denoiser,
}

struct Gpu {
    context: Rc<Context>,
    resources: Resources,
    buffers: Buffers,
    pipelines_factory: PipelinesFactory,
}

pub(crate) struct FrameBufferSettings {
    presentation_format: wgpu::TextureFormat,
    frame_buffer_size: FrameBufferSize,
    antialiasing_level: u32,
}

impl FrameBufferSettings {
    #[must_use]
    pub(crate) fn new(presentation_format: wgpu::TextureFormat, frame_buffer_size: FrameBufferSize, antialiasing_level: u32) -> Self {
        Self { presentation_format, frame_buffer_size, antialiasing_level }
    }
}

impl Renderer {
    const WORK_GROUP_SIZE_X: u32 = 8;
    const WORK_GROUP_SIZE_Y: u32 = 8;
    const WORK_GROUP_SIZE: Vector2<u32> = Vector2::new(Self::WORK_GROUP_SIZE_X, Self::WORK_GROUP_SIZE_Y);
    
    const BVH_INFLATION_RATE: f64 = 0.2;
    
    pub(crate) fn new(
        context: Rc<Context>,
        objects_container: VisualObjects,
        camera: Camera,
        frame_buffer_settings: FrameBufferSettings,
        strategy: RenderStrategyId,
        caches_path: Option<PathBuf>,
    )
        -> anyhow::Result<Self>
    {
        let pixel_side_subdivision: u32 = 1;
        let mut uniforms = Uniforms::new(frame_buffer_settings.frame_buffer_size, camera, pixel_side_subdivision);

        let scene = Hub::new(objects_container);

        let resources = Resources::new(context.clone());
        let buffers = Self::init_buffers(&scene, &context, &mut uniforms, &resources);
        let pipelines_factory = PipelinesFactory::new(context.clone(), frame_buffer_settings.presentation_format, caches_path);
        let mut gpu = Gpu { context, resources, buffers, pipelines_factory };

        let shader_source_text = scene.container().append_sdf_handling_code(WHOLE_TRACER_GPU_CODE);
        let shader_source_hash = seahash::hash(shader_source_text.as_bytes());

        let shader_module = gpu.resources.create_shader_module("ray tracer shader", shader_source_text.as_str());

        let monte_carlo_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "monte_carlo_code".to_string());
        let ray_tracing_monte_carlo = Rc::new(RefCell::new(
            Self::create_ray_tracing_pipeline(&mut gpu, &monte_carlo_code, ComputeRoutineEntryPoint::RayTracingMonteCarlo, false)));

        let deterministic_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "deterministic_code".to_string());
        let ray_tracing_deterministic = Rc::new(RefCell::new(
            Self::create_ray_tracing_pipeline(&mut gpu, &deterministic_code, ComputeRoutineEntryPoint::RayTracingDeterministic, true)));

        let surface_attributes_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "surface_attributes_pipeline_code".to_string());
        let surface_attributes = Self::create_surface_attributes_pipeline(&mut gpu, &surface_attributes_code);

        let default_strategy = ColorBufferEvaluationStrategy::new_monte_carlo(ray_tracing_monte_carlo.clone());
        let final_image_rasterization_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "final_image_rasterization_code".to_string());
        let final_image_rasterization = Self::create_rasterization_pipeline(&mut gpu, &final_image_rasterization_code, default_strategy.id());

        let mut renderer = Self {
            gpu,
            uniforms,
            pipeline_ray_tracing_monte_carlo: ray_tracing_monte_carlo.clone(),
            pipeline_ray_tracing_deterministic: ray_tracing_deterministic.clone(),
            color_buffer_evaluation: default_strategy,
            pipeline_surface_attributes: surface_attributes,
            pipeline_final_image_rasterization: final_image_rasterization,
            scene,

            #[cfg(feature = "denoiser")]
            denoiser: denoiser::Denoiser::new(),
        };
        renderer.set_render_strategy(strategy, frame_buffer_settings.antialiasing_level);
        
        Ok(renderer)
    }

    #[must_use]
    pub(crate) fn scene(&mut self) -> &mut Hub {
        &mut self.scene
    }
    
    pub(crate) fn set_render_strategy(&mut self, flavour: RenderStrategyId, antialiasing_level: u32) {
        if self.color_buffer_evaluation.id() == flavour {
            return;
        }
        
        self.color_buffer_evaluation = match flavour {
            RenderStrategyId::MonteCarlo => {
                ColorBufferEvaluationStrategy::new_monte_carlo(self.pipeline_ray_tracing_monte_carlo.clone())
            }
            RenderStrategyId::Deterministic => {
                ColorBufferEvaluationStrategy::new_deterministic(self.pipeline_ray_tracing_deterministic.clone())
            }
        };
        
        self.uniforms.reset_frame_accumulation(self.color_buffer_evaluation.frame_counter_default());
        self.uniforms.set_pixel_side_subdivision(antialiasing_level);
        Self::setup_frame_buffers_bindings_for_rasterization(&self.gpu, &mut self.pipeline_final_image_rasterization, flavour);
    }
    
    #[must_use]
    pub(crate) fn is_monte_carlo(&self) -> bool {
        1 == self.color_buffer_evaluation.frame_counter_increment()
    }

    #[must_use]
    fn update_buffer<T: GpuSerializationSize>(geometry_kind: &'static DataKind, buffer: &mut VersionedBuffer, resources: &Resources, scene: &VisualObjects, queue: &wgpu::Queue,) -> BufferUpdateStatus {
        let actual_data_version = scene.data_version(*geometry_kind);
        let serializer = || Self::serialize_scene_data::<T>(scene, geometry_kind);
        buffer.try_update_with_generator(actual_data_version, resources, queue, serializer)
    }
    
    #[must_use]
    fn serialize_triangles(scene: &VisualObjects) -> GpuReadySerializationBuffer {
        if scene.triangles_count() > 0 {
            scene.evaluate_serialized_triangles()
        } else {
            Self::make_empty_buffer_marker::<Triangle>()
        }
    }
    
    #[must_use]
    fn serialize_bvh(scene: &VisualObjects, aabb_inflation_rate: f64) -> (GpuReadySerializationBuffer, u32) {
        assert!(aabb_inflation_rate >= 0.0, "aabb_inflation is negative");
        if scene.bvh_inhabited() {
            let bvh = scene.evaluate_serialized_bvh(aabb_inflation_rate);
            let count = bvh.total_slots_count() as u32;
            (bvh, count)
        } else {
            (Self::make_empty_buffer_marker::<BvhNode>(), 0)
        }
    }
    
    #[must_use]
    fn update_buffers_if_scene_changed(&mut self) -> BuffersUpdateStatus {
        let container = self.scene.container();
        
        let mut composite_status = BuffersUpdateStatus::new();

        composite_status.merger_material(self.gpu.buffers.materials.try_update_with_generator(container.materials().data_version(), &self.gpu.resources, self.gpu.context.queue(), || container.materials().serialize()));
        
        composite_status.merge_geometry(Self::update_buffer::<Parallelogram>(&DataKind::Parallelogram, &mut self.gpu.buffers.parallelograms, &self.gpu.resources, container, self.gpu.context.queue()));
        self.uniforms.set_parallelograms_count(container.count_of_a_kind(DataKind::Parallelogram) as u32);
        
        let mut update_bvh = false;
        
        let triangles_set_version = container.data_version(DataKind::TriangleMesh);
        if self.gpu.buffers.triangles.version_diverges(triangles_set_version) {
            let serialized_triangles = Self::serialize_triangles(container);
            composite_status.merge_geometry(self.gpu.buffers.triangles.try_update_with_generator(triangles_set_version, &self.gpu.resources, self.gpu.context.queue(), || serialized_triangles));
            update_bvh = true;
        }

        let sdf_set_version = container.data_version(DataKind::Sdf);
        if self.gpu.buffers.sdf.version_diverges(sdf_set_version) {
            composite_status.merge_geometry(Self::update_buffer::<SdfInstance>(&DataKind::Sdf, &mut self.gpu.buffers.sdf, &self.gpu.resources, container, self.gpu.context.queue()));
            update_bvh = true;
        }

        if update_bvh {
            let (bvh, bvh_length) = Self::serialize_bvh(container, 0.0);
            composite_status.merge_bvh(self.gpu.buffers.bvh.update_with_generator(&self.gpu.resources, self.gpu.context.queue(), || bvh));

            let (bvh_inflated, bvh_inflated_length) = Self::serialize_bvh(container, Self::BVH_INFLATION_RATE);
            composite_status.merge_bvh(self.gpu.buffers.bvh_inflated.update_with_generator(&self.gpu.resources, self.gpu.context.queue(), || bvh_inflated));

            self.uniforms.set_bvh_length(bvh_length);
            assert_eq!(bvh_length, bvh_inflated_length);
        }
        
        let animator = self.scene.animator();
        if self.gpu.buffers.sdf_time.version_diverges(animator.version()) {
            let per_sdf_time = Self::make_gpu_ready_animation_times_array(animator);
            composite_status.merge_geometry(
                self.gpu.buffers.sdf_time.try_update_with_slice(animator.version(), &self.gpu.resources, self.gpu.context.queue(), &per_sdf_time)
            );
        }
        
        if composite_status.any_resized() {
            Self::create_geometry_buffers_bindings(&self.gpu, self.pipeline_ray_tracing_monte_carlo.borrow_mut().deref_mut(), false);
            Self::create_geometry_buffers_bindings(&self.gpu, self.pipeline_ray_tracing_deterministic.borrow_mut().deref_mut(), true);
            Self::create_geometry_buffers_bindings(&self.gpu, &mut self.pipeline_surface_attributes, false);
        }
        
        composite_status
    }
    
    #[must_use]
    fn make_gpu_ready_animation_times_array(animator: &TimeTracker) -> Vec<f32> {
        let mut per_sdf_time = vec![0.0_f32; std::cmp::max(1, animator.tracked_count())];
        animator.write_times(&mut per_sdf_time);
        per_sdf_time
    }
    
    #[must_use]
    fn make_empty_buffer_marker<T: GpuSerializationSize>() -> GpuReadySerializationBuffer {
        GpuReadySerializationBuffer::make_filled(1, T::SERIALIZED_QUARTET_COUNT, 0.0_f32)
    }
    
    #[must_use]
    fn serialize_scene_data<T: GpuSerializationSize>(scene: &VisualObjects, geometry_kind: &'static DataKind) -> GpuReadySerializationBuffer {
        if scene.count_of_a_kind(*geometry_kind) > 0 { 
            scene.evaluate_serialized(*geometry_kind) 
        } else {
            Self::make_empty_buffer_marker::<T>() 
        }
    }
    
    #[must_use]
    fn make_buffer<T: GpuSerializationSize>(scene: &VisualObjects, resources: &Resources, geometry_kind: &'static DataKind) -> VersionedBuffer {
        let serialized = Self::serialize_scene_data::<T>(scene, geometry_kind);
        VersionedBuffer::from_generator(scene.data_version(*geometry_kind), resources, geometry_kind.as_ref(), || serialized)
    }
    
    fn init_buffers(scene: &Hub, context: &Context, uniforms: &mut Uniforms, resources: &Resources) -> Buffers {
        let container = scene.container();
        let animator = scene.animator();
        
        let serialized_triangles = Self::serialize_triangles(container);

        let (bvh, bvh_length) = Self::serialize_bvh(container, 0.0);
        let (bvh_inflated, bvh_inflated_length) = Self::serialize_bvh(container, Self::BVH_INFLATION_RATE);
        assert_eq!(bvh_length, bvh_inflated_length);
        uniforms.set_bvh_length(bvh_length);

        let materials = if container.materials().count() > 0
            { container.materials().serialize() } else { Self::make_empty_buffer_marker::<Material>() };
        
        uniforms.set_parallelograms_count(container.count_of_a_kind(DataKind::Parallelogram) as u32);
        
        let per_sdf_time = Self::make_gpu_ready_animation_times_array(animator);
        
        Buffers {
            uniforms: resources.create_uniform_buffer("uniforms", uniforms.serialize().backend()),

            ray_tracing_frame_buffer: FrameBuffer::new(context.device(), uniforms.frame_buffer_size()),
            denoised_beauty_image: FrameBufferLayer::new(context.device(), uniforms.frame_buffer_size(), SupportUpdateFromCpu::Yes, "denoised pixels"),
            
            parallelograms: Self::make_buffer::<Parallelogram>(container, resources, &DataKind::Parallelogram),
            sdf: Self::make_buffer::<SdfInstance>(container, resources, &DataKind::Sdf),
            materials: VersionedBuffer::from_generator(container.materials().data_version(), resources, "materials", || materials),
            triangles: VersionedBuffer::from_generator(container.data_version(DataKind::TriangleMesh), resources, "triangles from all meshes", || serialized_triangles),
            
            bvh: ResizableBuffer::from_generator(resources, "bvh", || bvh),
            bvh_inflated: ResizableBuffer::from_generator(resources, "bvh inflated", || bvh_inflated),
            
            sdf_time: VersionedBuffer::from_slice(animator.version(), resources, "sdf time", &per_sdf_time),
        }
    }

    const UNIFORMS_GROUP_INDEX: u32 = 0;
    const FRAME_BUFFERS_GROUP_INDEX: u32 = 1;
    const SCENE_GROUP_INDEX: u32 = 2;

    #[must_use]
    fn create_surface_attributes_pipeline(gpu: &mut Gpu, code: &PipelineCode) -> ComputePipeline {
        let pipeline = gpu.pipelines_factory.create_compute_pipeline(ComputeRoutineEntryPoint::SurfaceAttributes, code);
        let uses_inflated_bvh = false;
        Self::create_compute_pipeline(gpu, pipeline, |device, buffers, pipeline| {
            Self::setup_frame_buffers_bindings_for_surface_attributes_compute(device, buffers, pipeline);
        }, uses_inflated_bvh)
    }
    
    #[must_use]
    fn create_ray_tracing_pipeline(gpu: &mut Gpu, code: &PipelineCode, routine: ComputeRoutineEntryPoint, uses_inflated_bvh: bool) -> ComputePipeline {
        let pipeline = gpu.pipelines_factory.create_compute_pipeline(routine, code);
        Self::create_compute_pipeline(gpu, pipeline, |device, buffers, pipeline| {
            Self::setup_frame_buffers_bindings_for_ray_tracing_compute(device, buffers, pipeline);
        }, uses_inflated_bvh)
    }

    #[must_use]
    fn create_compute_pipeline<Code>(gpu: &Gpu, pipeline: wgpu::ComputePipeline, customization: Code, uses_inflated_bvh: bool) -> ComputePipeline
        where Code: FnOnce(&wgpu::Device, &Buffers, &mut ComputePipeline), 
    {
        let device = gpu.context.device();
        let mut pipeline = ComputePipeline::new(pipeline);

        pipeline.setup_bind_group(Self::UNIFORMS_GROUP_INDEX, Some("compute pipeline uniform group"), device, |bind_group| {
            bind_group
                .add_entry(0, gpu.buffers.uniforms.clone())
            ;
        });

        customization(device, &gpu.buffers, &mut pipeline);

        Self::create_geometry_buffers_bindings(gpu, &mut pipeline, uses_inflated_bvh);
        
        pipeline
    }
    
    fn create_geometry_buffers_bindings(gpu: &Gpu, pipeline: &mut ComputePipeline, uses_inflated_bvh: bool) {
        let label = Some("compute pipeline scene group");
        pipeline.setup_bind_group(Self::SCENE_GROUP_INDEX, label, gpu.context.device(), |bind_group| {
            bind_group
                .add_entry(0, gpu.buffers.parallelograms.backend().clone())
                .add_entry(1, gpu.buffers.sdf.backend().clone())
                .add_entry(2, gpu.buffers.triangles.backend().clone())
                .add_entry(3, gpu.buffers.materials.backend().clone())
                .add_entry(4, gpu.buffers.bvh.backend().clone());
                
            if uses_inflated_bvh {
                bind_group.add_entry(5, gpu.buffers.bvh_inflated.backend().clone());
            }

            bind_group.add_entry(6, gpu.buffers.sdf_time.backend().clone());
        });
    }

    fn setup_frame_buffers_bindings_for_surface_attributes_compute(device: &wgpu::Device, buffers: &Buffers, surface_attributes_pipeline: &mut ComputePipeline) {
        let label = Some("'surface attributes' compute pipeline frame buffers group");

        surface_attributes_pipeline.setup_bind_group(Self::FRAME_BUFFERS_GROUP_INDEX, label, device, |bind_group_builder| {
            bind_group_builder
                .add_entry(1, buffers.ray_tracing_frame_buffer.object_id_at_gpu())
                .add_entry(2, buffers.ray_tracing_frame_buffer.normal_at_gpu())
                .add_entry(3, buffers.ray_tracing_frame_buffer.albedo_gpu())
            ;
        });
    }
    
    fn setup_frame_buffers_bindings_for_ray_tracing_compute(device: &wgpu::Device, buffers: &Buffers, ray_tracing_pipeline: &mut ComputePipeline) {
        let label = Some("ray tracing compute pipeline frame buffers group");

        ray_tracing_pipeline.setup_bind_group(Self::FRAME_BUFFERS_GROUP_INDEX, label, device, |bind_group_builder| {
            bind_group_builder
                .add_entry(0, buffers.ray_tracing_frame_buffer.noisy_pixel_color())
            ;
        });
    }

    fn create_rasterization_pipeline(gpu: &mut Gpu, code: &PipelineCode, render_strategy: RenderStrategyId) -> RasterizationPipeline {
        let pipeline = gpu.pipelines_factory.create_rasterization_pipeline(code);
        let mut rasterization_pipeline = RasterizationPipeline::new(pipeline);

        let uniforms_binding_index=  0;
        let bind_group_layout = rasterization_pipeline.bind_group_layout(uniforms_binding_index);
        let mut bind_group_builder = BindGroupBuilder::new(uniforms_binding_index, Some("rasterization pipeline uniform group"), bind_group_layout);
        bind_group_builder
            .add_entry(0, gpu.buffers.uniforms.clone())
        ;
        rasterization_pipeline.commit_bind_group(gpu.context.device(), bind_group_builder);

        Self::setup_frame_buffers_bindings_for_rasterization(gpu, &mut rasterization_pipeline, render_strategy);

        rasterization_pipeline
    }
    
    fn setup_frame_buffers_bindings_for_rasterization(gpu: &Gpu, rasterization_pipeline: &mut RasterizationPipeline, flavour: RenderStrategyId) {
        let label = Some("rasterization pipeline frame buffers group");

        let bind_group_layout = rasterization_pipeline.bind_group_layout(Self::FRAME_BUFFERS_GROUP_INDEX);

        let mut bind_group_builder = BindGroupBuilder::new(Self::FRAME_BUFFERS_GROUP_INDEX, label, bind_group_layout);
        
        if cfg!(feature = "denoiser") {
            if flavour == RenderStrategyId::Deterministic {
                bind_group_builder
                    .add_entry(0, gpu.buffers.ray_tracing_frame_buffer.noisy_pixel_color())
                ;   
            } else {
                bind_group_builder
                    .add_entry(0, gpu.buffers.denoised_beauty_image.gpu_render_target())
                ;
            }
        } else {
            bind_group_builder
                .add_entry(0, gpu.buffers.ray_tracing_frame_buffer.noisy_pixel_color())
            ;
        }
        
        rasterization_pipeline.commit_bind_group(gpu.context.device(), bind_group_builder);
    }

    pub(crate) fn set_output_size(&mut self, new_size: PhysicalSize<u32>) {
        let previous_frame_size = self.uniforms.frame_buffer_area();
        self.uniforms.set_frame_size(new_size);
        self.uniforms.reset_frame_accumulation(self.color_buffer_evaluation.frame_counter_default());
        
        let new_frame_size = self.uniforms.frame_buffer_area();
        if previous_frame_size < new_frame_size {
            let device = self.gpu.context.device();

            self.gpu.buffers.ray_tracing_frame_buffer = FrameBuffer::new(device, self.uniforms.frame_buffer_size());
            self.gpu.buffers.denoised_beauty_image = FrameBufferLayer::new(device, self.uniforms.frame_buffer_size(), SupportUpdateFromCpu::Yes, "denoised pixels");

            Self::setup_frame_buffers_bindings_for_ray_tracing_compute(device, &self.gpu.buffers, self.pipeline_ray_tracing_monte_carlo.borrow_mut().deref_mut());
            Self::setup_frame_buffers_bindings_for_ray_tracing_compute(device, &self.gpu.buffers, self.pipeline_ray_tracing_deterministic.borrow_mut().deref_mut());
            Self::setup_frame_buffers_bindings_for_surface_attributes_compute(device, &self.gpu.buffers, &mut self.pipeline_surface_attributes);
            Self::setup_frame_buffers_bindings_for_rasterization(&self.gpu, &mut self.pipeline_final_image_rasterization, self.color_buffer_evaluation.id());
        } else {
            self.gpu.buffers.ray_tracing_frame_buffer.invalidate_cpu_copies();
        }
    }

    #[must_use]
    pub(crate) fn object_in_pixel(&self, x: u32, y: u32) -> Option<ObjectUid> {
        let map = self.gpu.buffers.ray_tracing_frame_buffer.object_id_at_cpu();
        let index = (self.uniforms.frame_buffer_size().width() * y + x) as usize;
        assert!(index < map.len());
        let uid = map[index];
        
        if 0 == uid {
            return None;
        }
        
        Some(ObjectUid(uid))
    }

    pub(crate) fn start_new_frame(&mut self) {
        self.scene.update_time();
    }
    
    pub(crate) fn accumulate_more_rays(&mut self)  {
        let mut rebuild_geometry_buffers = self.gpu.buffers.ray_tracing_frame_buffer.object_id_at_cpu().is_empty();
        let scene_status = self.update_buffers_if_scene_changed();

        {
            let camera_changed = self.uniforms.mutable_camera().check_and_clear_updated_status();
            let geometry_changed = scene_status.geometry_updated();
            
            if scene_status.any_updated() {
                self.uniforms.reset_frame_accumulation(self.color_buffer_evaluation.frame_counter_default());
            }
            
            if camera_changed || geometry_changed {
                self.uniforms.reset_frame_accumulation(self.color_buffer_evaluation.frame_counter_default());
                rebuild_geometry_buffers = true;
            }
            
            self.uniforms.next_frame(self.color_buffer_evaluation.frame_counter_increment());
            
            // TODO: rewrite with 'write_buffer_with'? May be we need kind of ping-pong or circular buffer here?
            let uniform_values = self.uniforms.serialize();
            self.gpu.context.queue().write_buffer(&self.gpu.buffers.uniforms, 0, uniform_values.backend());
        }

        let rebuild_albedo_buffer = rebuild_geometry_buffers
            || self.gpu.buffers.ray_tracing_frame_buffer.albedo_at_cpu_is_absent()
            || scene_status.any_updated();

        let mut surface_properties_pass_or_none: Option<SubmissionIndex> = None;
        if rebuild_geometry_buffers || rebuild_albedo_buffer {
            let label = "nearest surface properties compute pass";
            let encoder = self.begin_compute_pass();
            surface_properties_pass_or_none = Some(
                self.compute_pass(encoder, label, &self.pipeline_surface_attributes, |pass| {
                    if rebuild_geometry_buffers {
                        if cfg!(feature = "denoiser") {
                            self.gpu.buffers.ray_tracing_frame_buffer.prepare_all_aux_buffers_copy_from_gpu(pass);
                        } else {
                            self.gpu.buffers.ray_tracing_frame_buffer.prepare_object_id_copy_from_gpu(pass);
                        }
                    } else if cfg!(feature = "denoiser") && rebuild_albedo_buffer {
                        self.gpu.buffers.ray_tracing_frame_buffer.prepare_albedo_copy_from_gpu(pass);
                    }
                })
            );
        }

        let label = "ray tracing compute pass";
        let mut encoder = self.begin_compute_pass();
        if (rebuild_geometry_buffers || scene_status.any_updated()) && self.color_buffer_evaluation.frame_counter_increment() > 0 {
            encoder.clear_buffer(self.gpu.buffers.ray_tracing_frame_buffer.noisy_pixel_color().as_ref(), BufferAddress::default(), None);
        }
        self.compute_pass(encoder, label, self.color_buffer_evaluation.pipeline().deref(), |pass|{
            if cfg!(feature = "denoiser") {
                self.prepare_pixel_color_copy_from_gpu(pass);   
            }
        });

        if surface_properties_pass_or_none.is_some() {
            if rebuild_geometry_buffers {
                if cfg!(feature = "denoiser") {
                    let copy_operation = self.gpu.buffers.ray_tracing_frame_buffer.copy_all_aux_buffers_from_gpu();
                    self.gpu.context.wait(surface_properties_pass_or_none);
                    pollster::block_on(copy_operation);
                } else {
                    let copy_operation = self.gpu.buffers.ray_tracing_frame_buffer.copy_object_id_from_gpu();
                    self.gpu.context.wait(surface_properties_pass_or_none);
                    pollster::block_on(copy_operation);
                }
            } else if cfg!(feature = "denoiser") && rebuild_albedo_buffer {
                let copy_operation = self.gpu.buffers.ray_tracing_frame_buffer.copy_albedo_from_gpu();
                self.gpu.context.wait(surface_properties_pass_or_none);
                pollster::block_on(copy_operation);
            }
        }
    }
    
    fn prepare_pixel_color_copy_from_gpu(&self, pass: &mut wgpu::CommandEncoder) {
        self.gpu.buffers.ray_tracing_frame_buffer.prepare_pixel_color_copy_from_gpu(pass);
    }

    #[cfg(any(test, feature = "denoiser"))]
    fn copy_noisy_pixels_to_cpu(&mut self) {
        let pixel_colors_buffer_gpu_to_cpu_transfer = self.gpu.buffers.ray_tracing_frame_buffer.copy_pixel_colors_from_gpu();
        self.gpu.context.wait(None);
        pollster::block_on(pixel_colors_buffer_gpu_to_cpu_transfer);
    }

    #[cfg(feature = "denoiser")]
    pub(crate) fn denoise_accumulated_image(&mut self, timer: &mut denoiser::MinMaxTimeMeasurer)
    {
        self.copy_noisy_pixels_to_cpu();

        {
            let frame_buffer_width = self.uniforms.frame_buffer_size().width() as usize;
            let frame_buffer_height = self.uniforms.frame_buffer_size().height() as usize;
            let (beauty, albedo, normal) = self.gpu.buffers.ray_tracing_frame_buffer.denoiser_input();
            let beauty_floats: &mut [f32] = bytemuck::cast_slice_mut(beauty);
            let albedo_floats: &[f32] = bytemuck::cast_slice(albedo);
            let normal_floats: &[f32] = bytemuck::cast_slice(normal);

            timer.start();
            let mut executor = self.denoiser.begin_denoise(frame_buffer_width, frame_buffer_height);
            executor.issue_albedo_write(albedo_floats);
            executor.issue_normal_write(normal_floats);
            executor.issue_noisy_beauty_write(beauty_floats);
            executor.filter(beauty_floats);
            timer.stop();
            
            self.gpu.buffers.denoised_beauty_image.fill_render_target(self.gpu.context.queue(), beauty);
            self.gpu.context.queue().submit([]);
        }
    }

    #[allow(dead_code)] 
    #[cfg(feature = "denoiser")]
    pub(crate) fn denoise_and_save(&mut self) {
        let divider = self.uniforms.frame_number() as f32;
        fn save(name: &str, width: usize, height: usize, data: &[PodVector], divider: f32,) {
            denoiser::write_rgba_file(denoiser::Path::new(format!("_exr_{}.exr", name).as_str()), width, height,
            |x,y| {
                let element = data[y * width + x];
                (
                    element.x / divider,
                    element.y / divider,
                    element.z / divider,
                    1.0,
                )
            }).unwrap();

            let mut data_cast = vec![0.0; width * height * 3];
            for y in 0..height {
                for x in 0..width {
                    let index = y * width + x;
                    data_cast[index * 3    ] = data[index].x / divider;
                    data_cast[index * 3 + 1] = data[index].y / divider;
                    data_cast[index * 3 + 2] = data[index].z / divider;
                }
            }
            let pfm = denoiser::PFMBuilder::new()
                .size(width, height)
                .color(true)
                .scale(-1.0)
                .data(data_cast)
                .build()
                .unwrap();
            let mut file = denoiser::File::create(format!("_pfm_{}.pfm", name).as_str()).unwrap();
            pfm.write_into(&mut file).unwrap();
        }

        let (beauty, albedo, normal) = self.gpu.buffers.ray_tracing_frame_buffer.denoiser_input();

        save("_beauty", self.uniforms.frame_buffer_size().width() as usize, self.uniforms.frame_buffer_size().height() as usize, beauty, divider);
        save("_albedo", self.uniforms.frame_buffer_size().width() as usize, self.uniforms.frame_buffer_size().height() as usize, albedo, 1.0);
        save("_normal", self.uniforms.frame_buffer_size().width() as usize, self.uniforms.frame_buffer_size().height() as usize, normal, 1.0);
    }
    
    pub(crate) fn present(&mut self, surface_texture: &wgpu::SurfaceTexture) {
        let view = &surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("rasterization pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0, }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };

        self.final_image_rasterization_pass(&mut render_pass_descriptor, &self.pipeline_final_image_rasterization,);
    }

    #[must_use]
    fn begin_compute_pass(&self) -> CommandEncoder {
        self.create_command_encoder("compute pass encoder")
    }

    fn compute_pass<CustomizationDelegate>(&self, encoder: CommandEncoder, label: &str, compute_pipeline: &ComputePipeline, customize: CustomizationDelegate) -> SubmissionIndex
    where CustomizationDelegate : FnOnce(&mut CommandEncoder){
        
        let mut encoder = encoder; {

            {let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });

            let work_groups_needed = self.uniforms.frame_buffer_size().work_groups_count(Self::WORK_GROUP_SIZE);
            compute_pipeline.set_into_pass(&mut pass);
            pass.dispatch_workgroups(work_groups_needed.x, work_groups_needed.y, 1);}
            
            customize(&mut encoder);
        }
        let command_buffer = encoder.finish();
        self.gpu.context.queue().submit(Some(command_buffer))
    }

    fn final_image_rasterization_pass(&self, rasterization_pass_descriptor: &mut wgpu::RenderPassDescriptor, rasterization_pipeline: &RasterizationPipeline) {
        let mut encoder = self.create_command_encoder("rasterization pass encoder"); {
            let mut rasterization_pass = encoder.begin_render_pass(rasterization_pass_descriptor);
            rasterization_pipeline.set_into_pass(&mut rasterization_pass);
            rasterization_pass.draw(0..6, 0..1); // TODO: magic const
        }
        let render_command_buffer = encoder.finish();
        self.gpu.context.queue().submit(Some(render_command_buffer));
    }

    #[must_use]
    fn create_command_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.gpu.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }

    #[must_use]
    pub fn camera(&mut self) -> &mut Camera {
        self.uniforms.mutable_camera()
    }
}

pub(crate) const WHOLE_TRACER_GPU_CODE: &str = include_str!("../../assets/shaders/tracer.wgsl");

struct Buffers {
    uniforms: Rc<wgpu::Buffer>,

    ray_tracing_frame_buffer: FrameBuffer,
    denoised_beauty_image: FrameBufferLayer<PodVector>,
    
    parallelograms: VersionedBuffer,
    sdf: VersionedBuffer,
    triangles: VersionedBuffer,
    materials: VersionedBuffer,

    bvh: ResizableBuffer,
    bvh_inflated: ResizableBuffer,
    
    sdf_time: VersionedBuffer,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use cgmath::{AbsDiffEq, SquareMatrix};
    use image::{ImageBuffer, Rgba};
    use std::fs;
    use std::path::Path;
    use crate::geometry::transform::Affine;
    #[cfg(feature = "denoiser")]
    use crate::serialization::pod_vector::PodVector;
    use crate::utils::tests::assert_utils::tests::assert_all_items_equal;
    use crate::utils::tests::common_values::tests::COMMON_PRESENTATION_FORMAT;
    #[cfg(feature = "denoiser")]
    use exr::prelude::write_rgba_file;
    use rstest::rstest;
    use crate::sdf::framework::code_generator::SdfRegistrator;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::object::sdf_box::SdfBox;

    const TEST_FRAME_BUFFER_WIDTH: u32 = 256;
    const TEST_FRAME_BUFFER_HEIGHT: u32 = 256;
    const TEST_FRAME_BUFFER_SIZE: FrameBufferSize = FrameBufferSize::new(TEST_FRAME_BUFFER_WIDTH, TEST_FRAME_BUFFER_HEIGHT);

    const TEST_COLOR_R: f32 = 0.25;
    const TEST_COLOR_G: f32 = 0.5;
    const TEST_COLOR_B: f32 = 1.0;

    #[must_use]
    fn make_render(scene: VisualObjects, camera: Camera, strategy: RenderStrategyId, antialiasing_level: u32, context: Rc<Context>) -> Renderer {
        let frame_buffer_settings = FrameBufferSettings::new(COMMON_PRESENTATION_FORMAT, TEST_FRAME_BUFFER_SIZE, antialiasing_level);
        Renderer::new(context.clone(), scene, camera, frame_buffer_settings, strategy, None)
            .expect("render instantiation has failed")
    }
    
    #[rstest]
    #[case(RenderStrategyId::MonteCarlo)]
    #[case(RenderStrategyId::Deterministic)]
    fn test_empty_scene_rendering(#[case] strategy: RenderStrategyId) {
        let camera = Camera::new_orthographic_camera(1.0, Point::new(0.0, 0.0, 0.0));
        let scene = VisualObjects::new(SdfRegistrator::default());
        let context = create_headless_wgpu_context();

        const ANTIALIASING_LEVEL: u32 = 1;
        let mut system_under_test = make_render(scene, camera, strategy, ANTIALIASING_LEVEL, context.clone());

        system_under_test.accumulate_more_rays();
        issue_frame_buffer_transfer_if_needed(context.clone(), &system_under_test);
        system_under_test.copy_noisy_pixels_to_cpu();
        
        assert_empty_color_buffer(&mut system_under_test);
        assert_empty_ids_buffer(&mut system_under_test);
        
        #[cfg(feature = "denoiser")]
        {
            assert_normals_and_albedo_are_empty(&mut system_under_test);
        }
    }
    
    #[test]
    fn test_single_parallelogram_rendering() {
        let camera = Camera::new_orthographic_camera(1.0, Point::new(0.0, 0.0, 0.0));
        
        let mut scene = VisualObjects::new(SdfRegistrator::default());
        let test_material = scene.materials_mutable().add(&Material::new().with_albedo(TEST_COLOR_R, TEST_COLOR_G, TEST_COLOR_B));
        
        scene.add_parallelogram(
            Point::new(-0.5, -0.5, 0.0), 
            Vector::new(1.0, 0.0, 0.0), 
            Vector::new(0.0, 1.0, 0.0), 
            test_material);

        let context = create_headless_wgpu_context();

        const ANTIALIASING_LEVEL: u32 = 1;
        let mut system_under_test = make_render(scene, camera, RenderStrategyId::MonteCarlo, ANTIALIASING_LEVEL, context.clone());

        system_under_test.accumulate_more_rays();
        
        assert_parallelogram_ids_in_center(&mut system_under_test, "single_parallelogram");
        
        #[cfg(feature = "denoiser")] 
        {
            assert_parallelogram_normals_in_center(&mut system_under_test);
            assert_parallelogram_albedo_in_center(&mut system_under_test);   
        }
    }

    #[test]
    fn test_single_box_sdf_rendering() {
        let camera = Camera::new_orthographic_camera(1.0, Point::new(0.0, 0.0, 0.0));

        let mut registrator = SdfRegistrator::default();
        let test_box_name = UniqueSdfClassName::new("specimen".to_string());
        registrator.add(&NamedSdf::new(SdfBox::new(Vector::new(0.5, 0.5, 0.5)), test_box_name.clone()));
        
        let mut scene = VisualObjects::new(registrator);
        let test_material = Material::new()
            .with_albedo(TEST_COLOR_R, TEST_COLOR_G, TEST_COLOR_B)
            .with_emission(TEST_COLOR_R, TEST_COLOR_G, TEST_COLOR_B);
        let test_material_uid = scene.materials_mutable().add(&test_material);
        scene.add_sdf(&Affine::identity(), 1.0, &test_box_name, test_material_uid);

        let context = create_headless_wgpu_context();

        const ANTIALIASING_LEVEL: u32 = 1;
        let mut system_under_test = make_render(scene, camera, RenderStrategyId::Deterministic, ANTIALIASING_LEVEL, context.clone());
        
        system_under_test.accumulate_more_rays();
        issue_frame_buffer_transfer_if_needed(context.clone(), &system_under_test);
        system_under_test.copy_noisy_pixels_to_cpu();
        
        assert_parallelogram_ids_in_center(&mut system_under_test, "sdf_box");
        assert_parallelogram_colors_in_center(&mut system_under_test, "sdf_box");
    }

    fn issue_frame_buffer_transfer_if_needed(context: Rc<Context>, render: &Renderer) {
        if cfg!(not(feature = "denoiser")) {
            let mut pass = context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            render.prepare_pixel_color_copy_from_gpu(&mut pass);
            context.queue().submit(Some(pass.finish()));
        }
    }

    #[cfg(feature = "denoiser")]
    fn assert_parallelogram_vector_data_in_center(data: &Vec<PodVector>, parallelogram: PodVector, background: PodVector, data_name: &str) {
        let exr_path = Path::new("tests").join(format!("out/{}.exr", data_name));

        write_rgba_file(exr_path, TEST_FRAME_BUFFER_WIDTH as usize, TEST_FRAME_BUFFER_HEIGHT as usize,
        |x,y| {
            let index = y * TEST_FRAME_BUFFER_WIDTH as usize + x;
            let element = data[index];
            (element.x, element.y, element.z, 1.0)
        }
        ).unwrap();

        assert_eq!(data.len(), TEST_FRAME_BUFFER_SIZE.area() as usize);
        
        assert_parallelogram_in_center(data, parallelogram, background, 
    |actual, expected, i, j| assert_eq!(actual, expected, "unexpected pixel value at ({i}, {j})"));
    }

    #[cfg(feature = "denoiser")]
    fn assert_parallelogram_albedo_in_center(system_under_test: &mut Renderer) {
        let (_, albedo, _) = system_under_test.gpu.buffers.ray_tracing_frame_buffer.denoiser_input();
        let parallelogram_color = PodVector { x: TEST_COLOR_R, y: TEST_COLOR_G, z: TEST_COLOR_B, w: 1.0 };
        let background_color = PodVector { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
        assert_parallelogram_vector_data_in_center(albedo, parallelogram_color, background_color, "single_parallelogram_colors");
    }

    #[cfg(feature = "denoiser")]
    fn assert_parallelogram_normals_in_center(system_under_test: &mut Renderer) {
        let (_, _, normal_map) = system_under_test.gpu.buffers.ray_tracing_frame_buffer.denoiser_input();
        let parallelogram_normal = PodVector { x: 0.0, y: 0.0, z: 1.0, w: 0.0 };
        let background_normal = PodVector { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
        assert_parallelogram_vector_data_in_center(normal_map, parallelogram_normal, background_normal, "single_parallelogram_normals");
    }

    fn assert_empty_color_buffer(system_under_test: &mut Renderer) {
        let colors = system_under_test.gpu.buffers.ray_tracing_frame_buffer.noisy_pixel_color_at_cpu();
        assert_all_items_equal(colors, PodVector { x: 0.1, y: 0.1, z: 0.1, w: 1.0 });
    }
    
    fn assert_empty_ids_buffer(system_under_test: &mut Renderer) {
        let object_id_map = system_under_test.gpu.buffers.ray_tracing_frame_buffer.object_id_at_cpu();
        assert_all_items_equal(object_id_map, 0);
    }

    #[cfg(feature = "denoiser")]
    fn assert_normals_and_albedo_are_empty(system_under_test: &mut Renderer) {
        let (_, albedo, normal_map) = system_under_test.gpu.buffers.ray_tracing_frame_buffer.denoiser_input();
        assert_all_items_equal(albedo, PodVector { x: 0.0, y: 0.0, z: 0.0, w: 1.0 });
        assert_all_items_equal(normal_map, PodVector { x: 0.0, y: 0.0, z: 0.0, w: 0.0 });
    }
    
    fn assert_parallelogram_ids_in_center(system_under_test: &mut Renderer, file_identity: &str) {
        let object_id_map = system_under_test.gpu.buffers.ray_tracing_frame_buffer.object_id_at_cpu();

        let png_path = Path::new("tests").join(format!("out/{file_identity}_identification.png"));
        save_u32_buffer_as_png(object_id_map, TEST_FRAME_BUFFER_WIDTH, TEST_FRAME_BUFFER_HEIGHT, png_path.clone());
        print_full_path(png_path.clone(), "object id map");
        
        assert_eq!(object_id_map.len(), TEST_FRAME_BUFFER_SIZE.area() as usize);

        let parallelogram_uid: u32 = 1;
        let null_uid: u32 = 0;
        assert_parallelogram_in_center(object_id_map, parallelogram_uid, null_uid, 
    |actual, expected, i, j| assert_eq!(actual, expected, "unexpected pixel value at ({i}, {j})"));
    }

    fn assert_parallelogram_colors_in_center(system_under_test: &mut Renderer, file_identity: &str) {
        let colors = system_under_test.gpu.buffers.ray_tracing_frame_buffer.noisy_pixel_color_at_cpu();

        let png_path = Path::new("tests").join(format!("out/{file_identity}_colors.png"));
        save_u32_buffer_as_png(&hdr_to_sdr(colors), TEST_FRAME_BUFFER_WIDTH, TEST_FRAME_BUFFER_HEIGHT, png_path.clone());
        print_full_path(png_path.clone(), "noisy colors");

        assert_eq!(colors.len(), TEST_FRAME_BUFFER_SIZE.area() as usize);

        let parallelogram_color = PodVector { x: 0.2697169, y: 0.5394338, z: 1.0788676, w: 1.0 };
        let null_color = PodVector { x: 0.1, y: 0.1, z: 0.1, w: 1.0 };
        assert_parallelogram_in_center(colors, parallelogram_color, null_color,
           |actual, expected, i, j| {
               if false == actual.abs_diff_eq(&expected, 0.1) {
                   panic!("unexpected pixel value at ({i}, {j}): actual = {actual}, expected = {expected}");
               }
           }
        );
    }

    fn hdr_to_sdr(input: &Vec<PodVector>) -> Vec<u32> {
        let mut result = Vec::<u32>::with_capacity(input.len());
        #[must_use] fn to_byte(channel: f32) -> u8 {
            (channel.clamp(0.0, 1.0) * u8::MAX as f32).clamp(0.0, 255.0) as u8
        }
        for color in input {
            let r = to_byte(color.x);
            let g = to_byte(color.y);
            let b = to_byte(color.z);
            let a = to_byte(color.w);
            result.push(((a as u32) << 24) | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32));
        }
        result
    }

    fn assert_parallelogram_in_center<AssertEquality, T>(data: &Vec<T>, parallelogram: T, background: T, assert_equality: AssertEquality) 
    where 
        T : Copy + PartialEq + std::fmt::Debug,
        AssertEquality: Fn(T, T, u32, u32),
    {
        let center_box_width = TEST_FRAME_BUFFER_WIDTH / 2;
        let center_box_height = TEST_FRAME_BUFFER_HEIGHT / 2;
        let center_box_left = (TEST_FRAME_BUFFER_WIDTH - center_box_width) / 2;
        let center_box_right = center_box_left + center_box_width;
        let center_box_top = (TEST_FRAME_BUFFER_HEIGHT - center_box_height) / 2;
        let center_box_bottom = center_box_top + center_box_height;

        for j in 0..TEST_FRAME_BUFFER_HEIGHT {
            for i in 0..TEST_FRAME_BUFFER_WIDTH {
                let pixel_index = TEST_FRAME_BUFFER_WIDTH * j + i;
                let actual = data[pixel_index as usize];
                let expected =
                    if i >= center_box_left && i < center_box_right
                        && j >= center_box_top && j < center_box_bottom {
                        parallelogram
                    } else {
                        background
                    };
                assert_equality(actual, expected, i, j);
            }
        }
    }

    fn print_full_path(path: impl AsRef<Path>, entity_name: &str) {
        match fs::canonicalize(path) {
            Ok(full_path) => println!("full path to '{}': {}", entity_name, full_path.display()),
            Err(e) => eprintln!("error: {}", e),
        }
    }

    fn save_u32_buffer_as_png(buffer: &Vec<u32>, image_width: u32, image_height: u32, path: impl AsRef<Path>) {
        let pixel_count = (image_width * image_height) as usize;
        assert!(buffer.len() >= pixel_count);

        let sliced = &buffer[..pixel_count];

        let raw_bytes: Vec<u8> = sliced
            .iter()
            .flat_map(|&px| px.to_ne_bytes())
            .collect();

        let buffer: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(image_width, image_height, raw_bytes.to_vec())
            .expect("failed to create image buffer");

        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).unwrap();
        }
        
        buffer.save(path.as_ref()).expect(format!("failed to save PNG into {}", path.as_ref().display()).as_str());
    }
}