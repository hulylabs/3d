use super::resources::{ComputeRoutine, Resources};
use crate::bvh::node::BvhNode;
use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::compute_pipeline::ComputePipeline;
use crate::gpu::context::Context;
use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::output::frame_buffer::FrameBuffer;
use crate::gpu::rasterization_pipeline::RasterizationPipeline;
use crate::objects::material::Material;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::SdfBox;
use crate::objects::sphere::Sphere;
use crate::objects::triangle::Triangle;
use crate::scene::camera::Camera;
use crate::scene::container::Container;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use bytemuck::checked::cast_slice;
use image::{ImageBuffer, Rgba};
use std::path::Path;
use std::rc::Rc;
use wgpu::wgt::PollType;
use wgpu::StoreOp;
use winit::dpi::PhysicalSize;

// TODO: work in progress

pub(crate) struct Renderer {
    context: Rc<Context>,
    resources: Resources,
    buffers: Buffers,
    uniforms: Uniforms,
    pipeline_ray_tracing: ComputePipeline,
    pipeline_object_id: ComputePipeline,
    pipeline_final_image_rasterization: RasterizationPipeline,
}

impl Renderer {
    pub(crate) fn new(
        context: Rc<Context>,
        scene_container: Container,
        camera: Camera,
        presentation_format: wgpu::TextureFormat,
        frame_buffer_size: FrameBufferSize,
    )
    -> anyhow::Result<Self> {
        let uniforms = Uniforms {
            frame_buffer_size,
            frame_number: 0,
            if_reset_framebuffer: false,
            camera,
        };

        let mut scene = scene_container;

        let resources = Resources::new(context.clone(), presentation_format);
        let buffers = Self::init_buffers(&mut scene, &context, &uniforms, &resources, uniforms.frame_buffer_size);
        
        let ray_tracing_shader_module = resources.create_shader_module("ray tracer shader", CODE_FOR_GPU)?;
        let ray_tracing = Self::create_ray_tracing_pipeline(&context, &resources, &buffers, &ray_tracing_shader_module);
        
        let object_id_shader_module = resources.create_shader_module("object id shader", CODE_FOR_GPU)?;
        let object_id = Self::create_object_id_pipeline(&context, &resources, &buffers, &object_id_shader_module);
        
        let final_image_rasterization = Self::create_rasterization_pipeline(&context, &resources, &buffers, &ray_tracing_shader_module);

        Ok(Self {
            context,
            resources,
            buffers,
            uniforms,
            pipeline_ray_tracing: ray_tracing,
            pipeline_object_id: object_id,
            pipeline_final_image_rasterization: final_image_rasterization,
        })
    }

    fn init_buffers(scene: &mut Container, context: &Context, uniforms: &Uniforms, resources: &Resources, frame_buffer_size: FrameBufferSize) -> Buffers {
        let empty_triangles_marker
            = GpuReadySerializationBuffer::make_filled(1, Triangle::SERIALIZED_QUARTET_COUNT, -1.0_f32);
        let empty_bvh_marker
            = GpuReadySerializationBuffer::make_filled(1, BvhNode::SERIALIZED_QUARTET_COUNT, -1.0_f32);

        let serialized_triangles = scene.evaluate_serialized_triangles();
        
        let triangles = if serialized_triangles.empty()
        { &empty_triangles_marker } else { serialized_triangles.geometry() };
        let bvh = if serialized_triangles.empty()
        { &empty_bvh_marker } else { serialized_triangles.bvh() };

        let materials = if scene.materials_count() > 0
            { scene.evaluate_serialized_materials() } else { GpuReadySerializationBuffer::make_filled(1, Material::SERIALIZED_QUARTET_COUNT, 0.0_f32) };

        let spheres = if scene.spheres_count() > 0
            { scene.evaluate_serialized_spheres() } else { GpuReadySerializationBuffer::make_filled(1, Sphere::SERIALIZED_QUARTET_COUNT, -1.0_f32) };

        let parallelograms = if scene.parallelograms_count() > 0
            { scene.evaluate_serialized_parallelograms() } else { GpuReadySerializationBuffer::make_filled(1, Parallelogram::SERIALIZED_QUARTET_COUNT, -1.0_f32) };

        let sdf = if scene.sdf_objects_count() > 0
            { scene.evaluate_serialized_sdf() } else { GpuReadySerializationBuffer::make_filled(1, SdfBox::SERIALIZED_QUARTET_COUNT, -1.0_f32) };

        // TODO: we can use inline defined shader data, rather than this complication
        let full_screen_quad_vertices: Vec<f32> = vec![-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0];

        Buffers {
            uniforms: resources.create_uniform_buffer("uniforms", uniforms.serialize().backend()),

            frame_buffer: FrameBuffer::new(context.device(), frame_buffer_size),

            spheres: resources.create_storage_buffer_write_only("spheres", spheres.backend()),
            parallelograms: resources.create_storage_buffer_write_only("parallelograms", parallelograms.backend()),
            sdf: resources.create_storage_buffer_write_only("sdf", sdf.backend()),
            triangles: resources.create_storage_buffer_write_only("triangles from all meshes", triangles.backend()),
            materials: resources.create_storage_buffer_write_only("materials", materials.backend()),
            bvh: resources.create_storage_buffer_write_only("bvh", bvh.backend()),
            vertex: resources.create_vertex_buffer("full screen quad vertices", cast_slice(&full_screen_quad_vertices)),
        }
    }

    const UNIFORMS_GROUP_INDEX: u32 = 0;
    const FRAME_BUFFERS_GROUP_INDEX: u32 = 1;
    const SCENE_GROUP_INDEX: u32 = 2;

    #[must_use]
    fn create_object_id_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> ComputePipeline {
        let pipeline = resources.create_compute_pipeline(ComputeRoutine::ShaderObjectIdEntryPoint, module);
        Self::create_compute_pipeline(context, buffers, pipeline, |context, buffers, pipeline| {
            Self::create_frame_buffers_bindings_for_object_id_compute(context, buffers, pipeline);
        })
    }
    
    #[must_use]
    fn create_ray_tracing_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> ComputePipeline {
        let pipeline = resources.create_compute_pipeline(ComputeRoutine::ShaderRayTracingEntryPoint, module);
        Self::create_compute_pipeline(context, buffers, pipeline, |context, buffers, pipeline| {
            Self::create_frame_buffers_bindings_for_ray_tracing_compute(context, buffers, pipeline);
        })
    }

    #[must_use]
    fn create_compute_pipeline<Code>(context: &Context, buffers: &Buffers, pipeline: wgpu::ComputePipeline, customization: Code) -> ComputePipeline
        where Code: FnOnce(&Context, &Buffers, &mut ComputePipeline), {
        let mut pipeline = ComputePipeline::new(pipeline);

        pipeline.setup_bind_group(Self::UNIFORMS_GROUP_INDEX, Some("compute pipeline uniform group"), context.device(), |bind_group| {
            bind_group
                .add_entry(0, buffers.uniforms.clone())
            ;
        });

        customization(context, buffers, &mut pipeline);

        pipeline.setup_bind_group(Self::SCENE_GROUP_INDEX, Some("compute pipeline scene group"), context.device(), |bind_group| {
            bind_group
                .add_entry(0, buffers.spheres.clone())
                .add_entry(1, buffers.parallelograms.clone())
                .add_entry(2, buffers.sdf.clone())
                .add_entry(3, buffers.triangles.clone())
                .add_entry(4, buffers.materials.clone())
                .add_entry(5, buffers.bvh.clone())
            ;
        });

        pipeline
    }

    fn create_frame_buffers_bindings_for_object_id_compute(context: &Context, buffers: &Buffers, object_id_pipeline: &mut ComputePipeline) {
        let label = Some("object id compute pipeline frame buffers group");

        object_id_pipeline.setup_bind_group(Self::FRAME_BUFFERS_GROUP_INDEX, label, context.device(), |bind_group_builder| {
            bind_group_builder
                .add_entry(1, buffers.frame_buffer.object_id_at_gpu())
            ;
        });
    }
    
    fn create_frame_buffers_bindings_for_ray_tracing_compute(context: &Context, buffers: &Buffers, ray_tracing_pipeline: &mut ComputePipeline) {
        let label = Some("ray tracing compute pipeline frame buffers group");

        ray_tracing_pipeline.setup_bind_group(Self::FRAME_BUFFERS_GROUP_INDEX, label, context.device(), |bind_group_builder| {
            bind_group_builder
                .add_entry(0, buffers.frame_buffer.pixel_color())
            ;
        });
    }

    fn create_rasterization_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> RasterizationPipeline {
        let mut rasterization_pipeline = RasterizationPipeline::new(resources.create_rasterization_pipeline(module));

        let uniforms_binding_index=  0;
        let bind_group_layout = rasterization_pipeline.bind_group_layout(uniforms_binding_index);
        let mut bind_group_builder = BindGroupBuilder::new(uniforms_binding_index, Some("rasterization pipeline uniform group"), bind_group_layout);
        bind_group_builder
            .add_entry(0, buffers.uniforms.clone())
        ;
        rasterization_pipeline.commit_bind_group(context.device(), bind_group_builder);

        Self::create_frame_buffers_bindings_for_rasterization(context, buffers, &mut rasterization_pipeline);

        rasterization_pipeline
    }

    fn create_frame_buffers_bindings_for_rasterization(context: &Context, buffers: &Buffers, rasterization_pipeline: &mut RasterizationPipeline) {
        let label = Some("rasterization pipeline frame buffers group");

        let bind_group_layout = rasterization_pipeline.bind_group_layout(Self::FRAME_BUFFERS_GROUP_INDEX);

        let mut bind_group_builder = BindGroupBuilder::new(Self::FRAME_BUFFERS_GROUP_INDEX, label, bind_group_layout);
        bind_group_builder
            .add_entry(0, buffers.frame_buffer.pixel_color())
        ;

        rasterization_pipeline.commit_bind_group(context.device(), bind_group_builder);
    }

    pub(crate) fn set_output_size(&mut self, new_size: PhysicalSize<u32>) {
        let previous_frame_size = self.uniforms.frame_buffer_area();
        self.uniforms.set_frame_size(new_size);
        let new_frame_size = self.uniforms.frame_buffer_area();

        if previous_frame_size < new_frame_size {
            self.buffers.frame_buffer = FrameBuffer::new(&self.context.device(), self.uniforms.frame_buffer_size);
            
            Self::create_frame_buffers_bindings_for_ray_tracing_compute(&self.context, &self.buffers, &mut self.pipeline_ray_tracing);
            Self::create_frame_buffers_bindings_for_object_id_compute(&self.context, &self.buffers, &mut self.pipeline_object_id);
            Self::create_frame_buffers_bindings_for_rasterization(&self.context, &self.buffers, &mut self.pipeline_final_image_rasterization);
        } else {
            self.buffers.frame_buffer.invalidate();
        }
    }

    pub(crate) fn accumulate_more_rays(&mut self)  {
        let mut rebuild_object_id_buffer = self.buffers.frame_buffer.object_id_at_cpu().is_empty();
        {
            if self.uniforms.camera.check_and_clear_updated_status() {
                self.uniforms.reset_frame_accumulation();
                rebuild_object_id_buffer = true;
            }
            self.uniforms.next_frame();

            // TODO: rewrite with 'write_buffer_with'? May be we need kind of ping-pong or circular buffer here?
            let uniform_values = self.uniforms.serialize();
            self.context.queue().write_buffer(&self.buffers.uniforms, 0, uniform_values.backend());

            self.uniforms.drop_reset_flag();
        }
        
        self.compute_pass("ray tracing compute pass", &self.pipeline_ray_tracing, |_|{});

        if rebuild_object_id_buffer {
            self.compute_pass("object id compute pass", &self.pipeline_object_id, |after_pass| {
                self.buffers.frame_buffer.prepare_object_id_copy_from_gpu(after_pass);
            });
            let object_id_buffer_gpu_to_cpu_transfer = self.buffers.frame_buffer.copy_object_id_from_gpu();
            self.context.device().poll(PollType::Wait).expect("failed to poll the device");
            pollster::block_on(object_id_buffer_gpu_to_cpu_transfer);
        }
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

        self.final_image_rasterization_pass(&mut render_pass_descriptor, &self.pipeline_final_image_rasterization, &self.buffers.vertex);
    }

    fn compute_pass<CustomizationDelegate>(&self, label: &str, compute_pipeline: &ComputePipeline, customize: CustomizationDelegate)
        where CustomizationDelegate : FnOnce(&mut wgpu::CommandEncoder) {
        let work_groups_needed = self.uniforms.frame_buffer_size.area() / 64; // TODO: 64?
        let mut encoder = self.create_command_encoder("compute pass encoder"); {

            {let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None, // TODO: what can be used for?
            });

            compute_pipeline.set_into_pass(&mut pass);
            pass.dispatch_workgroups(work_groups_needed, 1, 1);}
            
            customize(&mut encoder);
        }
        let command_buffer = encoder.finish();
        self.context.queue().submit(Some(command_buffer));
    }

    fn final_image_rasterization_pass(&self, rasterization_pass_descriptor: &mut wgpu::RenderPassDescriptor, rasterization_pipeline: &RasterizationPipeline, vertex_buffer: &wgpu::Buffer) {
        let mut encoder = self.create_command_encoder("rasterization pass encoder"); {
            let mut rasterization_pass = encoder.begin_render_pass(rasterization_pass_descriptor);
            rasterization_pipeline.set_into_pass(&mut rasterization_pass);
            rasterization_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rasterization_pass.draw(0..6, 0..1); // TODO: magic const
        }
        let render_command_buffer = encoder.finish();
        self.context.queue().submit(Some(render_command_buffer));
    }

    fn create_command_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.context.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }

    #[must_use]
    pub fn camera(&mut self) -> &mut Camera {
        &mut self.uniforms.camera
    }
}

pub(crate) const CODE_FOR_GPU: &str = include_str!("../../assets/shaders/tracer.wgsl");

struct Buffers {
    uniforms: Rc<wgpu::Buffer>,

    frame_buffer: FrameBuffer,

    spheres: Rc<wgpu::Buffer>,
    parallelograms: Rc<wgpu::Buffer>,
    sdf: Rc<wgpu::Buffer>,
    triangles: Rc<wgpu::Buffer>,
    materials: Rc<wgpu::Buffer>,
    bvh: Rc<wgpu::Buffer>,
    vertex: Rc<wgpu::Buffer>,
}

struct Uniforms {
    frame_buffer_size: FrameBufferSize,
    frame_number: u32,
    if_reset_framebuffer: bool,
    camera: Camera,
}

impl Uniforms {
    fn reset_frame_accumulation(&mut self) {
        self.if_reset_framebuffer = true;
        self.frame_number = 0;
    }

    fn drop_reset_flag(&mut self) {
        self.if_reset_framebuffer = false;
    }

    fn set_frame_size(&mut self, new_size: PhysicalSize<u32>) {
        self.frame_buffer_size = FrameBufferSize::new(new_size.width, new_size.height);
        self.reset_frame_accumulation();
    }

    fn next_frame(&mut self) {
        self.frame_number += 1;
    }

    #[must_use]
    fn frame_buffer_area(&self) -> u32 {
        self.frame_buffer_size.area()
    }

    const SERIALIZED_QUARTET_COUNT: usize = 1 + Camera::SERIALIZED_QUARTET_COUNT;

    #[must_use]
    fn serialize(&self) -> GpuReadySerializationBuffer {
        let mut result = GpuReadySerializationBuffer::new(1, Self::SERIALIZED_QUARTET_COUNT);

        result.write_quartet_f32(
            self.frame_buffer_size.width() as f32,
            self.frame_buffer_size.height() as f32,
            self.frame_number as f32,
            if self.if_reset_framebuffer { 1.0 } else { 0.0 },
        );
        self.camera.serialize_into(&mut result);
        debug_assert!(result.object_fully_written());

        result
    }
}

fn save_u32_buffer_as_png(buffer: &Vec<u32>, image_width: u32, image_height: u32, path: &Path) {
    let pixel_count = (image_width * image_height) as usize;
    assert!(buffer.len() >= pixel_count);

    let sliced = &buffer[..pixel_count];

    let raw_bytes: Vec<u8> = sliced
        .iter()
        .flat_map(|&px| px.to_ne_bytes())
        .collect();

    let buffer: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(image_width, image_height, raw_bytes.to_vec())
        .expect("failed to create image buffer");

    buffer.save(path).expect(format!("failed to save PNG into {}", path.display()).as_str());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::gpu::headless_device::tests::create_headless_wgpu_context;
    use cgmath::EuclideanSpace;
    use std::fs;
    use std::path::Path;
    use wgpu::TextureFormat;

    const DEFAULT_FRAME_WIDTH: u32 = 800;
    const DEFAULT_FRAME_HEIGHT: u32 = 600;

    #[must_use]
    fn make_system_under_test() -> Uniforms {
        let frame_buffer_size = FrameBufferSize::new(DEFAULT_FRAME_WIDTH, DEFAULT_FRAME_HEIGHT);
        let camera = Camera::new_perspective_camera(1.0, Point::origin());

        Uniforms {
            frame_buffer_size,
            frame_number: 0,
            if_reset_framebuffer: false,
            camera,
        }
    }

    const SLOT_FRAME_WIDTH: usize = 0;
    const SLOT_FRAME_HEIGHT: usize = 1;
    const SLOT_FRAME_NUMBER: usize = 2;
    const SLOT_RESET_FRAME_BUFFER: usize = 3;

    #[test]
    fn test_reset_frame_accumulation() {
        let mut system_under_test = make_system_under_test();

        system_under_test.next_frame();
        system_under_test.reset_frame_accumulation();

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 0.0);
        assert_eq!(actual_state_floats[SLOT_RESET_FRAME_BUFFER], 1.0);
    }

    #[test]
    fn test_drop_reset_flag() {
        let mut system_under_test = make_system_under_test();

        system_under_test.reset_frame_accumulation();
        system_under_test.drop_reset_flag();

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_RESET_FRAME_BUFFER], 0.0);
    }

    #[test]
    fn test_set_frame_size() {
        let expected_width = 1024;
        let expected_height = 768;
        let new_size = PhysicalSize::new(expected_width, expected_height);
        let mut system_under_test = make_system_under_test();

        system_under_test.set_frame_size(new_size);

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_WIDTH], expected_width as f32);
        assert_eq!(actual_state_floats[SLOT_FRAME_HEIGHT], expected_height as f32);
    }

    #[test]
    fn test_next_frame() {
        let mut system_under_test = make_system_under_test();

        system_under_test.next_frame();
        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 1.0);

        system_under_test.next_frame();
        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 2.0);
    }

    #[test]
    fn test_frame_buffer_area() {
        let system_under_test = make_system_under_test();

        let expected_area = DEFAULT_FRAME_WIDTH * DEFAULT_FRAME_HEIGHT;
        assert_eq!(system_under_test.frame_buffer_area(), expected_area);
    }

    #[test]
    fn test_serialize() {
        let system_under_test = make_system_under_test();

        let actual_state = system_under_test.serialize();
        let actual_state_floats: &[f32] = bytemuck::cast_slice(&actual_state.backend());

        assert_eq!(actual_state_floats[SLOT_FRAME_WIDTH], 800.0);
        assert_eq!(actual_state_floats[SLOT_FRAME_HEIGHT], 600.0);
        assert_eq!(actual_state_floats[SLOT_FRAME_NUMBER], 0.0);
        assert_eq!(actual_state_floats[SLOT_RESET_FRAME_BUFFER], 0.0);
    }

    #[test]
    fn test_single_parallelogram_rendering() {
        let camera = Camera::new_orthographic_camera(1.0, Point::new(0.0, 0.0, 0.0));
        
        let mut scene = Container::new();
        let dummy_material = scene.add_material(&Material::new());
        
        scene.add_parallelogram(Point::new(-0.5, -0.5, 0.0), Vector::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0), dummy_material);

        let frame_buffer_width = 256;
        let frame_buffer_height = 256;
        let frame_buffer_size = FrameBufferSize::new(frame_buffer_width, frame_buffer_height);
        let context = create_headless_wgpu_context();

        let mut system_under_test 
            = Renderer::new(context.clone(), scene, camera, TextureFormat::Rgba8Unorm, frame_buffer_size)
                .expect("render instantiation has failed");

        system_under_test.accumulate_more_rays();
        let object_id_map = system_under_test.buffers.frame_buffer.object_id_at_cpu();
        assert_eq!(object_id_map.len(), frame_buffer_size.area() as usize);
        
        let center_pixel = frame_buffer_width * (frame_buffer_height / 2) + frame_buffer_width / 2;
        let actual_uid = object_id_map[center_pixel as usize];
        let expected_uid = 1;
        assert_eq!(expected_uid, actual_uid);
        
        let png_path = Path::new("./single_parallelogram_identification.png");
        save_u32_buffer_as_png(object_id_map, frame_buffer_width, frame_buffer_height, png_path);
        print_full_path(png_path, "object id map");
    }

    fn print_full_path(path: &Path, entity_name: &str) {
        match fs::canonicalize(path) {
            Ok(full_path) => println!("full path to '{}': {}", entity_name, full_path.display()),
            Err(e) => eprintln!("error: {}", e),
        }
    }
}