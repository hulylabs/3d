use crate::bvh::node::BvhNode;
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
use crate::gpu::resources::Resources;
use crate::gpu::versioned_buffer::{BufferUpdateStatus, VersionedBuffer};
use crate::objects::material::Material;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::SdfInstance;
use crate::objects::triangle::Triangle;
use crate::scene::camera::Camera;
use crate::scene::container::{Container, DataKind};
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::pod_vector::PodVector;
use crate::serialization::serializable_for_gpu::GpuSerializationSize;
use crate::utils::object_uid::ObjectUid;
use cgmath::Vector2;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::rc::Rc;
use wgpu::wgt::PollType;
use wgpu::StoreOp;
use winit::dpi::PhysicalSize;
use crate::gpu::resizable_buffer::ResizableBuffer;

#[cfg(feature = "denoiser")]
mod denoiser {
    pub(super) use crate::denoiser::entry::Denoiser;
    pub(super) use crate::utils::min_max_time_measurer::MinMaxTimeMeasurer;
    pub(super) use exr::prelude::write_rgba_file;
    pub(super) use pxm::PFMBuilder;
    pub(super) use std::fs::File;
    pub(super) use std::path::Path;
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

pub(crate) struct Renderer {
    gpu: Gpu,
    uniforms: Uniforms,
    pipeline_ray_tracing_monte_carlo: Rc<RefCell<ComputePipeline>>,
    pipeline_ray_tracing_deterministic: Rc<RefCell<ComputePipeline>>,
    color_buffer_evaluation: ColorBufferEvaluationStrategy,
    pipeline_surface_attributes: ComputePipeline,
    pipeline_final_image_rasterization: RasterizationPipeline,
    scene: Container,
    
    #[cfg(feature = "denoiser")]
    denoiser: denoiser::Denoiser,
}

impl Renderer {
    const WORK_GROUP_SIZE_X: u32 = 8;
    const WORK_GROUP_SIZE_Y: u32 = 8;
    const WORK_GROUP_SIZE: Vector2<u32> = Vector2::new(Self::WORK_GROUP_SIZE_X, Self::WORK_GROUP_SIZE_Y);
    
    const BVH_INFLATION_RATE: f64 = 0.1;
    
    pub(crate) fn new(
        context: Rc<Context>,
        scene_container: Container,
        camera: Camera,
        frame_buffer_settings: FrameBufferSettings,
        strategy: RenderStrategyId,
        caches_path: Option<PathBuf>,
    )
        -> anyhow::Result<Self>
    {
        let mut uniforms = Uniforms {
            frame_buffer_size: frame_buffer_settings.frame_buffer_size,
            frame_number: 0,
            if_reset_framebuffer: false,
            camera,
            parallelograms_count: 0,
            sdf_count: 0,
            bvh_length: 0,
            pixel_side_subdivision: 1,
        };

        let mut scene = scene_container;

        let resources = Resources::new(context.clone());
        let buffers = Self::init_buffers(&mut scene, &context, &mut uniforms, &resources);
        let pipelines_factory = PipelinesFactory::new(context.clone(), frame_buffer_settings.presentation_format, caches_path);
        let mut gpu = Gpu { context, resources, buffers, pipelines_factory };

        let shader_source_text = scene.append_sdf_handling_code(WHOLE_TRACER_GPU_CODE);
        let shader_source_hash = seahash::hash(shader_source_text.as_bytes());

        let shader_module = gpu.resources.create_shader_module("ray tracer shader", shader_source_text.as_str());

        let monte_carlo_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "monte_carlo_code".to_string());
        let ray_tracing_monte_carlo = Rc::new(RefCell::new(Self::create_ray_tracing_pipeline(&mut gpu, &monte_carlo_code, ComputeRoutineEntryPoint::RayTracingMonteCarlo)));

        let deterministic_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "deterministic_code".to_string());
        let ray_tracing_deterministic = Rc::new(RefCell::new(Self::create_ray_tracing_pipeline(&mut gpu, &deterministic_code, ComputeRoutineEntryPoint::RayTracingDeterministic)));

        let object_id_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "object_id_pipeline_code".to_string());
        let object_id = Self::create_object_id_pipeline(&mut gpu, &object_id_code);

        let default_strategy = ColorBufferEvaluationStrategy::new_monte_carlo(ray_tracing_monte_carlo.clone());
        let final_image_rasterization_code = PipelineCode::new(shader_module.clone(), shader_source_hash, "final_image_rasterization_code".to_string());
        let final_image_rasterization = Self::create_rasterization_pipeline(&mut gpu, &final_image_rasterization_code, default_strategy.id());

        let mut renderer = Self {
            gpu,
            uniforms,
            pipeline_ray_tracing_monte_carlo: ray_tracing_monte_carlo.clone(),
            pipeline_ray_tracing_deterministic: ray_tracing_deterministic.clone(),
            color_buffer_evaluation: default_strategy,
            pipeline_surface_attributes: object_id,
            pipeline_final_image_rasterization: final_image_rasterization,
            scene,

            #[cfg(feature = "denoiser")]
            denoiser: denoiser::Denoiser::new(),
        };
        renderer.set_render_strategy(strategy, frame_buffer_settings.antialiasing_level);
        
        Ok(renderer)
    }

    #[must_use]
    pub(crate) fn scene(&mut self) -> &mut Container {
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
    fn update_buffer<T: GpuSerializationSize>(geometry_kind: &'static DataKind, buffer: &mut VersionedBuffer, resources: &Resources, scene: &Container, queue: &wgpu::Queue,) -> BufferUpdateStatus {
        let actual_data_version = scene.data_version(*geometry_kind);
        let serializer = || Self::serialize_scene_data::<T>(scene, geometry_kind);
        buffer.try_update(actual_data_version, resources, queue, serializer)
    }
    
    #[must_use]
    fn serialize_triangles(scene: &Container) -> GpuReadySerializationBuffer {
        if scene.triangles_count() > 0 {
            scene.evaluate_serialized_triangles()
        } else {
            Self::make_empty_buffer_marker::<Triangle>()
        }
    }
    
    #[must_use]
    fn serialize_bvh(scene: &Container, aabb_inflation_rate: f64) -> (GpuReadySerializationBuffer, u32) {
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
        let mut composite_status = BuffersUpdateStatus::new();

        composite_status.merger_material(self.gpu.buffers.materials.try_update(self.scene.materials().data_version(), &self.gpu.resources, self.gpu.context.queue(), || self.scene.materials().serialize()));
        
        composite_status.merge_geometry(Self::update_buffer::<Parallelogram>(&DataKind::Parallelogram, &mut self.gpu.buffers.parallelograms, &self.gpu.resources, &self.scene, self.gpu.context.queue()));
        self.uniforms.parallelograms_count = self.scene.count_of_a_kind(DataKind::Parallelogram) as u32;
        
        let mut update_bvh = false;
        
        let triangles_set_version = self.scene.data_version(DataKind::TriangleMesh);
        if self.gpu.buffers.triangles.version_diverges(triangles_set_version) {
            let serialized_triangles = Self::serialize_triangles(self.scene());
            composite_status.merge_geometry(self.gpu.buffers.triangles.try_update(triangles_set_version, &self.gpu.resources, self.gpu.context.queue(), || serialized_triangles));
            update_bvh = true;
        }

        let sdf_set_version = self.scene.data_version(DataKind::Sdf);
        if self.gpu.buffers.sdf.version_diverges(sdf_set_version) {
            composite_status.merge_geometry(Self::update_buffer::<SdfInstance>(&DataKind::Sdf, &mut self.gpu.buffers.sdf, &self.gpu.resources, &self.scene, self.gpu.context.queue()));
            self.uniforms.sdf_count = self.scene.count_of_a_kind(DataKind::Sdf) as u32;
            update_bvh = true;
        }

        if update_bvh {
            let (bvh, bvh_length) = Self::serialize_bvh(self.scene(), 0.0);
            composite_status.merge_bvh(self.gpu.buffers.bvh.update(&self.gpu.resources, self.gpu.context.queue(), || bvh));

            let (bvh_inflated, bvh_inflated_length) = Self::serialize_bvh(self.scene(), Self::BVH_INFLATION_RATE);
            composite_status.merge_bvh(self.gpu.buffers.bvh_inflated.update(&self.gpu.resources, self.gpu.context.queue(), || bvh_inflated));

            self.uniforms.bvh_length = bvh_length;
            assert_eq!(bvh_length, bvh_inflated_length);
        }
        
        if composite_status.any_resized() {
            Self::create_geometry_buffers_bindings(&self.gpu, self.pipeline_ray_tracing_monte_carlo.borrow_mut().deref_mut());
            Self::create_geometry_buffers_bindings(&self.gpu, self.pipeline_ray_tracing_deterministic.borrow_mut().deref_mut());
            Self::create_geometry_buffers_bindings(&self.gpu, &mut self.pipeline_surface_attributes);
        }
        
        composite_status
    }
    
    #[must_use]
    fn make_empty_buffer_marker<T: GpuSerializationSize>() -> GpuReadySerializationBuffer {
        GpuReadySerializationBuffer::make_filled(1, T::SERIALIZED_QUARTET_COUNT, 0.0_f32)
    }
    
    #[must_use]
    fn serialize_scene_data<T: GpuSerializationSize>(scene: &Container, geometry_kind: &'static DataKind) -> GpuReadySerializationBuffer {
        if scene.count_of_a_kind(*geometry_kind) > 0 { 
            scene.evaluate_serialized(*geometry_kind) 
        } else {
            Self::make_empty_buffer_marker::<T>() 
        }
    }
    
    #[must_use]
    fn make_buffer<T: GpuSerializationSize>(scene: &Container, resources: &Resources, geometry_kind: &'static DataKind) -> VersionedBuffer {
        let serialized = Self::serialize_scene_data::<T>(scene, geometry_kind);
        VersionedBuffer::new(scene.data_version(*geometry_kind), resources, geometry_kind.as_ref(), || serialized)
    }
    
    fn init_buffers(scene: &mut Container, context: &Context, uniforms: &mut Uniforms, resources: &Resources) -> Buffers {
        let serialized_triangles = Self::serialize_triangles(scene);

        let (bvh, bvh_length) = Self::serialize_bvh(scene, 0.0);
        let (bvh_inflated, bvh_inflated_length) = Self::serialize_bvh(scene, Self::BVH_INFLATION_RATE);
        assert_eq!(bvh_length, bvh_inflated_length);
        uniforms.bvh_length = bvh_length;

        let materials = if scene.materials().count() > 0
            { scene.materials().serialize() } else { Self::make_empty_buffer_marker::<Material>() };
        
        uniforms.parallelograms_count = scene.count_of_a_kind(DataKind::Parallelogram) as u32;
        uniforms.sdf_count = scene.count_of_a_kind(DataKind::Sdf) as u32;
        
        Buffers {
            uniforms: resources.create_uniform_buffer("uniforms", uniforms.serialize().backend()),

            ray_tracing_frame_buffer: FrameBuffer::new(context.device(), uniforms.frame_buffer_size),
            denoised_beauty_image: FrameBufferLayer::new(context.device(), uniforms.frame_buffer_size, SupportUpdateFromCpu::Yes, "denoised pixels"),
            
            parallelograms: Self::make_buffer::<Parallelogram>(scene, resources, &DataKind::Parallelogram),
            sdf: Self::make_buffer::<SdfInstance>(scene, resources, &DataKind::Sdf),
            materials: VersionedBuffer::new(scene.materials().data_version(), resources, "materials", || materials),
            triangles: VersionedBuffer::new(scene.data_version(DataKind::TriangleMesh), resources, "triangles from all meshes", || serialized_triangles),
            bvh: ResizableBuffer::new(resources,"bvh", || bvh),
            bvh_inflated: ResizableBuffer::new(resources,"bvh inflated", || bvh_inflated),
        }
    }

    const UNIFORMS_GROUP_INDEX: u32 = 0;
    const FRAME_BUFFERS_GROUP_INDEX: u32 = 1;
    const SCENE_GROUP_INDEX: u32 = 2;

    #[must_use]
    fn create_object_id_pipeline(gpu: &mut Gpu, code: &PipelineCode) -> ComputePipeline {
        let pipeline = gpu.pipelines_factory.create_compute_pipeline(ComputeRoutineEntryPoint::ObjectId, code);
        Self::create_compute_pipeline(gpu, pipeline, |device, buffers, pipeline| {
            Self::setup_frame_buffers_bindings_for_surface_attributes_compute(device, buffers, pipeline);
        })
    }
    
    #[must_use]
    fn create_ray_tracing_pipeline(gpu: &mut Gpu, code: &PipelineCode, routine: ComputeRoutineEntryPoint) -> ComputePipeline {
        let pipeline = gpu.pipelines_factory.create_compute_pipeline(routine, code);
        Self::create_compute_pipeline(gpu, pipeline, |device, buffers, pipeline| {
            Self::setup_frame_buffers_bindings_for_ray_tracing_compute(device, buffers, pipeline);
        })
    }

    #[must_use]
    fn create_compute_pipeline<Code>(gpu: &Gpu, pipeline: wgpu::ComputePipeline, customization: Code) -> ComputePipeline
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

        Self::create_geometry_buffers_bindings(gpu, &mut pipeline);
        
        pipeline
    }
    
    fn create_geometry_buffers_bindings(gpu: &Gpu, pipeline: &mut ComputePipeline) {
        pipeline.setup_bind_group(Self::SCENE_GROUP_INDEX, Some("compute pipeline scene group"), gpu.context.device(), |bind_group| {
            bind_group
                .add_entry(0, gpu.buffers.parallelograms.backend().clone())
                .add_entry(1, gpu.buffers.sdf.backend().clone())
                .add_entry(2, gpu.buffers.triangles.backend().clone())
                .add_entry(3, gpu.buffers.materials.backend().clone())
                .add_entry(4, gpu.buffers.bvh.backend().clone())
                //.add_entry(5, gpu.buffers.bvh_inflated.backend().clone())
            ;
        });
    }

    fn setup_frame_buffers_bindings_for_surface_attributes_compute(device: &wgpu::Device, buffers: &Buffers, surface_attributes_pipeline: &mut ComputePipeline) {
        let label = Some("object id compute pipeline frame buffers group");

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

            self.gpu.buffers.ray_tracing_frame_buffer = FrameBuffer::new(device, self.uniforms.frame_buffer_size);
            self.gpu.buffers.denoised_beauty_image = FrameBufferLayer::new(device, self.uniforms.frame_buffer_size, SupportUpdateFromCpu::Yes, "denoised pixels");

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
        let index = (self.uniforms.frame_buffer_size.width() * y + x) as usize;
        assert!(index < map.len());
        let uid = map[index];
        
        if 0 == uid {
            return None;
        }
        
        Some(ObjectUid(uid))
    }

    pub(crate) fn accumulate_more_rays(&mut self)  {
        let mut rebuild_geometry_buffers = self.gpu.buffers.ray_tracing_frame_buffer.object_id_at_cpu().is_empty();
        let scene_status = self.update_buffers_if_scene_changed();

        {
            let camera_changed = self.uniforms.camera.check_and_clear_updated_status();
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
            self.uniforms.drop_reset_flag();
        }

        let rebuild_albedo_buffer = rebuild_geometry_buffers
            || self.gpu.buffers.ray_tracing_frame_buffer.albedo_at_cpu_is_absent()
            || scene_status.any_updated();
        
        self.compute_pass("ray tracing compute pass", self.color_buffer_evaluation.pipeline().deref(), |ray_tracing_pass|{
            self.gpu.buffers.ray_tracing_frame_buffer.prepare_pixel_color_copy_from_gpu(ray_tracing_pass);
        });

        if rebuild_geometry_buffers || rebuild_albedo_buffer {
            self.compute_pass("nearest surface properties compute pass", &self.pipeline_surface_attributes, |after_pass| {
                if rebuild_geometry_buffers {
                    self.gpu.buffers.ray_tracing_frame_buffer.prepare_aux_buffers_copy_from_gpu(after_pass);
                } else if rebuild_albedo_buffer {
                    self.gpu.buffers.ray_tracing_frame_buffer.prepare_albedo_copy_from_gpu(after_pass);
                }
            });
            
            if rebuild_geometry_buffers {
                let copy_operation = self.gpu.buffers.ray_tracing_frame_buffer.copy_aux_buffers_from_gpu();
                self.gpu.context.device().poll(PollType::Wait).expect("failed to poll the device");
                pollster::block_on(copy_operation);
            } else if rebuild_albedo_buffer {
                let copy_operation = self.gpu.buffers.ray_tracing_frame_buffer.copy_albedo_from_gpu();
                self.gpu.context.device().poll(PollType::Wait).expect("failed to poll the device");
                pollster::block_on(copy_operation);
            }
        }
    }

    #[cfg(any(test, feature = "denoiser"))]
    fn copy_noisy_pixels_to_cpu(&mut self) {
        let pixel_colors_buffer_gpu_to_cpu_transfer = self.gpu.buffers.ray_tracing_frame_buffer.copy_pixel_colors_from_gpu();
        self.gpu.context.device().poll(PollType::Wait).expect("failed to poll the device");
        pollster::block_on(pixel_colors_buffer_gpu_to_cpu_transfer);
    }

    #[cfg(feature = "denoiser")]
    pub(crate) fn denoise_accumulated_image(&mut self, timer: &mut denoiser::MinMaxTimeMeasurer)
    {
        self.copy_noisy_pixels_to_cpu();

        {
            let frame_buffer_width = self.uniforms.frame_buffer_size.width() as usize;
            let frame_buffer_height = self.uniforms.frame_buffer_size.height() as usize;
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
        let divider = self.uniforms.frame_number as f32;
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

        save("_beauty", self.uniforms.frame_buffer_size.width() as usize, self.uniforms.frame_buffer_size.height() as usize, beauty, divider);
        save("_albedo", self.uniforms.frame_buffer_size.width() as usize, self.uniforms.frame_buffer_size.height() as usize, albedo, 1.0);
        save("_normal", self.uniforms.frame_buffer_size.width() as usize, self.uniforms.frame_buffer_size.height() as usize, normal, 1.0);
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

    fn compute_pass<CustomizationDelegate>(&self, label: &str, compute_pipeline: &ComputePipeline, customize: CustomizationDelegate)
        where CustomizationDelegate : FnOnce(&mut wgpu::CommandEncoder) {
        
        let mut encoder = self.create_command_encoder("compute pass encoder"); {

            {let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });

            let work_groups_needed = self.uniforms.frame_buffer_size.work_groups_count(Self::WORK_GROUP_SIZE);
            compute_pipeline.set_into_pass(&mut pass);
            pass.dispatch_workgroups(work_groups_needed.x, work_groups_needed.y, 1);}
            
            customize(&mut encoder);
        }
        let command_buffer = encoder.finish();
        self.gpu.context.queue().submit(Some(command_buffer));
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
        &mut self.uniforms.camera
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
}

struct Uniforms {
    frame_buffer_size: FrameBufferSize,
    frame_number: u32,
    if_reset_framebuffer: bool,
    camera: Camera,
    
    parallelograms_count: u32,
    sdf_count: u32,
    bvh_length: u32,
    pixel_side_subdivision: u32,
}

impl Uniforms {
    fn reset_frame_accumulation(&mut self, value: u32) {
        self.if_reset_framebuffer = true;
        self.frame_number = value;
    }

    fn drop_reset_flag(&mut self) {
        self.if_reset_framebuffer = false;
    }

    fn set_frame_size(&mut self, new_size: PhysicalSize<u32>) {
        self.frame_buffer_size = FrameBufferSize::new(new_size.width, new_size.height);
    }

    fn next_frame(&mut self, increment: u32) {
        self.frame_number += increment;
    }

    #[must_use]
    fn frame_buffer_area(&self) -> u32 {
        self.frame_buffer_size.area()
    }

    fn set_pixel_side_subdivision(&mut self, level: u32) {
        let level: u32 = if 0 == level { 1 } else { level };
        self.pixel_side_subdivision = level;
    }

    const SERIALIZED_QUARTET_COUNT: usize = 3 + Camera::SERIALIZED_QUARTET_COUNT;

    #[must_use]
    fn serialize(&self) -> GpuReadySerializationBuffer {
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
           if self.if_reset_framebuffer { 1.0 } else { 0.0 },
        );
        
        self.camera.serialize_into(&mut result);

        result.write_quartet_u32(self.parallelograms_count, self.sdf_count, self.bvh_length, self.pixel_side_subdivision);
        
        debug_assert!(result.object_fully_written());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use cgmath::{AbsDiffEq, EuclideanSpace, SquareMatrix};
    use image::{ImageBuffer, Rgba};
    use std::fs;
    use std::path::Path;

    use crate::geometry::transform::Affine;
    use crate::sdf::code_generator::SdfRegistrator;
    use crate::sdf::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::sdf_box::SdfBox;
    #[cfg(feature = "denoiser")]
    use crate::serialization::pod_vector::PodVector;
    use crate::utils::tests::assert_utils::tests::assert_all_items_equal;
    use crate::utils::tests::common_values::tests::COMMON_PRESENTATION_FORMAT;
    #[cfg(feature = "denoiser")]
    use exr::prelude::write_rgba_file;
    use rstest::rstest;

    const DEFAULT_FRAME_WIDTH: u32 = 800;
    const DEFAULT_FRAME_HEIGHT: u32 = 600;

    const DEFAULT_PARALLELOGRAMS_COUNT: u32 = 5;
    const DEFAULT_SDF_COUNT: u32 = 6;
    const DEFAULT_BVH_LENGTH: u32 = 8;
    const DEFAULT_PIXEL_SIDE_SUBDIVISION: u32 = 4;

    #[must_use]
    fn make_test_uniforms_instance() -> Uniforms {
        let frame_buffer_size = FrameBufferSize::new(DEFAULT_FRAME_WIDTH, DEFAULT_FRAME_HEIGHT);
        let camera = Camera::new_perspective_camera(1.0, Point::origin());

        Uniforms {
            frame_buffer_size,
            frame_number: 0,
            if_reset_framebuffer: false,
            camera,
            
            parallelograms_count: DEFAULT_PARALLELOGRAMS_COUNT,
            sdf_count: DEFAULT_SDF_COUNT,
            bvh_length: DEFAULT_BVH_LENGTH,
            pixel_side_subdivision: DEFAULT_PIXEL_SIDE_SUBDIVISION,
        }
    }

    const SLOT_FRAME_WIDTH: usize = 0;
    const SLOT_FRAME_HEIGHT: usize = 1;
    const SLOT_FRAME_AREA: usize = 2;
    const SLOT_FRAME_ASPECT: usize = 3;

    const SLOT_FRAME_INVERTED_WIDTH: usize = 4;
    const SLOT_FRAME_INVERTED_HEIGHT: usize = 5;
    const SLOT_FRAME_NUMBER: usize = 6;
    const SLOT_RESET_FRAME_BUFFER: usize = 7;

    const SLOT_PARALLELOGRAMS_COUNT: usize = 40;
    const SLOT_SDF_COUNT: usize = 41;
    const SLOT_BVH_LENGTH: usize = 42;
    const SLOT_PIXEL_SIDE_SUBDIVISION: usize = 43;

    #[test]
    fn test_hash() {
        println!("{}", seahash::hash("test_string".as_bytes()));
        println!("{}", seahash::hash("test_string".as_bytes()));
        println!("{}", seahash::hash("test_string".as_bytes()));
        println!("{}", seahash::hash("test_string".as_bytes()));
    }

    #[test]
    fn test_uniforms_reset_frame_accumulation() {
        let mut system_under_test = make_test_uniforms_instance();

        system_under_test.next_frame(1);
        system_under_test.reset_frame_accumulation(0);

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 0.0);
        assert_eq!(actual_state_floats[SLOT_RESET_FRAME_BUFFER], 1.0);
    }

    #[test]
    fn test_uniforms_drop_reset_flag() {
        let mut system_under_test = make_test_uniforms_instance();

        system_under_test.reset_frame_accumulation(0);
        system_under_test.drop_reset_flag();

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_RESET_FRAME_BUFFER], 0.0);
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
        let system_under_test = make_test_uniforms_instance();

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_WIDTH].to_bits(), DEFAULT_FRAME_WIDTH);
        assert_eq!(actual_state_floats[SLOT_FRAME_HEIGHT].to_bits(), DEFAULT_FRAME_HEIGHT);
        assert_eq!(actual_state_floats[SLOT_FRAME_AREA].to_bits(), DEFAULT_FRAME_WIDTH * DEFAULT_FRAME_HEIGHT);
        assert_eq!(actual_state_floats[SLOT_FRAME_ASPECT], DEFAULT_FRAME_WIDTH as f32 / DEFAULT_FRAME_HEIGHT as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_INVERTED_WIDTH], 1.0 / DEFAULT_FRAME_WIDTH as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_INVERTED_HEIGHT], 1.0 / DEFAULT_FRAME_HEIGHT as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 0.0);
        assert_eq!(actual_state_floats[SLOT_RESET_FRAME_BUFFER], 0.0);
        
        assert_eq!(actual_state_floats[SLOT_PARALLELOGRAMS_COUNT].to_bits(), DEFAULT_PARALLELOGRAMS_COUNT);
        assert_eq!(actual_state_floats[SLOT_SDF_COUNT].to_bits(), DEFAULT_SDF_COUNT);
        assert_eq!(actual_state_floats[SLOT_BVH_LENGTH].to_bits(), DEFAULT_BVH_LENGTH);
        assert_eq!(actual_state_floats[SLOT_PIXEL_SIDE_SUBDIVISION].to_bits(), DEFAULT_PIXEL_SIDE_SUBDIVISION);
    }

    const TEST_FRAME_BUFFER_WIDTH: u32 = 256;
    const TEST_FRAME_BUFFER_HEIGHT: u32 = 256;
    const TEST_FRAME_BUFFER_SIZE: FrameBufferSize = FrameBufferSize::new(TEST_FRAME_BUFFER_WIDTH, TEST_FRAME_BUFFER_HEIGHT);

    const TEST_COLOR_R: f32 = 0.25;
    const TEST_COLOR_G: f32 = 0.5;
    const TEST_COLOR_B: f32 = 1.0;

    #[must_use]
    fn make_render(scene: Container, camera: Camera, strategy: RenderStrategyId, antialiasing_level: u32, context: Rc<Context>) -> Renderer {
        let frame_buffer_settings = FrameBufferSettings::new(COMMON_PRESENTATION_FORMAT, TEST_FRAME_BUFFER_SIZE, antialiasing_level);
        Renderer::new(context.clone(), scene, camera, frame_buffer_settings, strategy, None)
            .expect("render instantiation has failed")
    }
    
    #[rstest]
    #[case(RenderStrategyId::MonteCarlo)]
    #[case(RenderStrategyId::Deterministic)]
    fn test_empty_scene_rendering(#[case] strategy: RenderStrategyId) {
        let camera = Camera::new_orthographic_camera(1.0, Point::new(0.0, 0.0, 0.0));
        let scene = Container::new(SdfRegistrator::default());
        let context = create_headless_wgpu_context();

        const ANTIALIASING_LEVEL: u32 = 1;
        let mut system_under_test = make_render(scene, camera, strategy, ANTIALIASING_LEVEL, context.clone());

        system_under_test.accumulate_more_rays();
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
        
        let mut scene = Container::new(SdfRegistrator::default());
        let test_material = scene.materials().add(&Material::new().with_albedo(TEST_COLOR_R, TEST_COLOR_G, TEST_COLOR_B));
        
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
        
        let mut scene = Container::new(registrator);
        let test_material = Material::new()
            .with_albedo(TEST_COLOR_R, TEST_COLOR_G, TEST_COLOR_B)
            .with_emission(TEST_COLOR_R, TEST_COLOR_G, TEST_COLOR_B);
        let test_material_uid = scene.materials().add(&test_material);
        scene.add_sdf(&Affine::identity(), &test_box_name, test_material_uid);

        let context = create_headless_wgpu_context();

        const ANTIALIASING_LEVEL: u32 = 1;
        let mut system_under_test = make_render(scene, camera, RenderStrategyId::Deterministic, ANTIALIASING_LEVEL, context.clone());
        
        system_under_test.accumulate_more_rays();
        system_under_test.copy_noisy_pixels_to_cpu();
        
        assert_parallelogram_ids_in_center(&mut system_under_test, "sdf_box");
        assert_parallelogram_colors_in_center(&mut system_under_test, "sdf_box");
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