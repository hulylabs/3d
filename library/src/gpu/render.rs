use super::resources::Resources;
use crate::bvh::node::BvhNode;
use crate::gpu::bind_group_builder::BindGroupBuilder;
use crate::gpu::compute_pipeline::ComputePipeline;
use crate::gpu::context::Context;
use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::rasterization_pipeline::RasterizationPipeline;
use crate::objects::material::Material;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sdf::SdfBox;
use crate::objects::sphere::Sphere;
use crate::objects::triangle::Triangle;
use crate::objects::triangle_mesh::TriangleMesh;
use crate::scene::camera::Camera;
use crate::scene::container::Container;
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use bytemuck::checked::cast_slice;
use std::rc::Rc;
use wgpu::StoreOp;
use winit::dpi::PhysicalSize;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

// TODO: work in progress

pub(crate) struct Renderer {
    context: Rc<Context>,
    resources: Resources,
    buffers: Buffers,
    uniforms: Uniforms,
    pipeline_ray_tracing: ComputePipeline,
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
        let shader_module = resources.create_shader_module("tracer shader", CODE_FOR_GPU)?;
        let buffers = Self::init_buffers(&mut scene, &uniforms, &resources, uniforms.frame_buffer_size);
        let ray_tracing = Self::create_ray_tracing_pipeline(&context, &resources, &buffers, &shader_module);
        let final_image_rasterization = Self::create_rasterization_pipeline(&context, &resources, &buffers, &shader_module);

        Ok(Self {
            context,
            resources,
            buffers,
            uniforms,
            pipeline_ray_tracing: ray_tracing,
            pipeline_final_image_rasterization: final_image_rasterization,
        })
    }

    fn init_buffers(scene: &mut Container, uniforms: &Uniforms, resources: &Resources, frame_buffer_size: FrameBufferSize) -> Buffers {
        let empty_meshes_marker
            = GpuReadySerializationBuffer::make_filled(1, TriangleMesh::SERIALIZED_QUARTET_COUNT, -1.0_f32);
        let empty_triangles_marker
            = GpuReadySerializationBuffer::make_filled(1, Triangle::SERIALIZED_QUARTET_COUNT, -1.0_f32);
        let empty_bvh_marker
            = GpuReadySerializationBuffer::make_filled(1, BvhNode::SERIALIZED_QUARTET_COUNT, -1.0_f32);

        let serialized_triangles = scene.evaluate_serialized_triangles();
        let meshes = if serialized_triangles.empty()
        { &empty_meshes_marker } else { serialized_triangles.meshes() };
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

            pixel_color_buffer: resources.create_pixel_color_buffer(frame_buffer_size),
            object_id_buffer: resources.create_object_id_buffer(frame_buffer_size),

            spheres: resources.create_storage_buffer_write_only("spheres", spheres.backend()),
            parallelograms: resources.create_storage_buffer_write_only("parallelograms", parallelograms.backend()),
            sdf: resources.create_storage_buffer_write_only("sdf", sdf.backend()),
            triangles: resources.create_storage_buffer_write_only("triangles from all meshes", triangles.backend()),
            meshes: resources.create_storage_buffer_write_only("meshes meta data", meshes.backend()),
            materials: resources.create_storage_buffer_write_only("materials", materials.backend()),
            bvh: resources.create_storage_buffer_write_only("bvh", bvh.backend()),
            vertex: resources.create_vertex_buffer("full screen quad vertices", cast_slice(&full_screen_quad_vertices)),
        }
    }

    const UNIFORMS_GROUP_INDEX: u32 = 0;
    const FRAME_BUFFERS_GROUP_INDEX: u32 = 1;
    const SCENE_GROUP_INDEX: u32 = 2;

    fn create_ray_tracing_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> ComputePipeline {
        let mut ray_tracing_pipeline = ComputePipeline::new(resources.create_ray_tracing_pipeline(module));

        ray_tracing_pipeline.setup_bind_group(Self::UNIFORMS_GROUP_INDEX, Some("compute pipeline uniform group"), context.device(), |bind_group|
        {
            bind_group
                .add_entry(0, buffers.uniforms.clone())
            ;
        });

        Self::create_frame_buffers_bindings_for_compute(context, buffers, &mut ray_tracing_pipeline);

        ray_tracing_pipeline.setup_bind_group(Self::SCENE_GROUP_INDEX, Some("compute pipeline scene group"), context.device(), |bind_group|
        {
            bind_group
                .add_entry(0, buffers.spheres.clone())
                .add_entry(1, buffers.parallelograms.clone())
                .add_entry(2, buffers.sdf.clone())
                .add_entry(3, buffers.triangles.clone())
                .add_entry(4, buffers.meshes.clone())
                .add_entry(5, buffers.materials.clone())
                .add_entry(6, buffers.bvh.clone())
            ;
        });

        ray_tracing_pipeline
    }

    fn create_frame_buffers_bindings_for_compute(context: &Context, buffers: &Buffers, compute_pipeline: &mut ComputePipeline) {
        let label = Some("compute pipeline frame buffers group");

        compute_pipeline.setup_bind_group(Self::FRAME_BUFFERS_GROUP_INDEX, label, context.device(), |bind_group_builder|
        {
            bind_group_builder
                .add_entry(0, buffers.pixel_color_buffer.clone())
                //.add_entry(1, buffers.object_id_buffer.clone())
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
            .add_entry(0, buffers.pixel_color_buffer.clone())
        ;

        rasterization_pipeline.commit_bind_group(context.device(), bind_group_builder);
    }

    pub(crate) fn set_output_size(&mut self, new_size: PhysicalSize<u32>) {
        let previous_frame_size = self.uniforms.frame_buffer_area();
        self.uniforms.set_frame_size(new_size);
        let new_frame_size = self.uniforms.frame_buffer_area();

        if previous_frame_size < new_frame_size {
            self.buffers.pixel_color_buffer = self.resources.create_pixel_color_buffer(self.uniforms.frame_buffer_size);
            self.buffers.object_id_buffer = self.resources.create_object_id_buffer(self.uniforms.frame_buffer_size);

            Self::create_frame_buffers_bindings_for_compute(&self.context, &self.buffers, &mut self.pipeline_ray_tracing);
            Self::create_frame_buffers_bindings_for_rasterization(&self.context, &self.buffers, &mut self.pipeline_final_image_rasterization);
        }
    }

    pub(crate) fn execute(&mut self, surface_texture: &wgpu::SurfaceTexture)  {
        {
            self.uniforms.next_frame();

            if self.uniforms.camera.check_and_clear_updated_status() {
                self.uniforms.reset_frame_accumulation();
                self.uniforms.next_frame();
            }

            // TODO: rewrite with 'write_buffer_with'. May be we need kind of ping-pong or circular buffer here?
            let uniform_values = self.uniforms.serialize();
            self.context.queue().write_buffer(&self.buffers.uniforms, 0, uniform_values.backend());

            self.uniforms.drop_reset_flag();
        }

        let work_groups_needed = self.uniforms.frame_buffer_size.area() / 64; // TODO: 64?
        self.ray_tracing_pass(&self.pipeline_ray_tracing, work_groups_needed);

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

        self.final_iamge_rasterization_pass(&mut render_pass_descriptor, &self.pipeline_final_image_rasterization, &self.buffers.vertex);
    }

    fn ray_tracing_pass(&self, compute_pipeline: &ComputePipeline, work_groups_needed: u32) {
        let mut encoder = self.create_command_encoder("compute pass encoder");
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor
            {
                label: Some("ray tracing compute pass"),
                timestamp_writes: None, // TODO: what can be used for?
            });

            compute_pipeline.set_into_pass(&mut pass);
            pass.dispatch_workgroups(work_groups_needed, 1, 1);
        }
        let command_buffer = encoder.finish();
        self.context.queue().submit(Some(command_buffer));
    }

    fn final_iamge_rasterization_pass(&self, rasterization_pass_descriptor: &mut wgpu::RenderPassDescriptor, rasterization_pipeline: &RasterizationPipeline, vertex_buffer: &wgpu::Buffer) {
        let mut encoder = self.create_command_encoder("rasterization pass encoder");
        {
            let mut rasterization_pass = encoder.begin_render_pass(rasterization_pass_descriptor);
            rasterization_pipeline.set_into_pass(&mut rasterization_pass);
            rasterization_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rasterization_pass.draw(0..6, 0..1);
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

    pixel_color_buffer: Rc<wgpu::Buffer>,
    object_id_buffer: Rc<wgpu::Buffer>,

    spheres: Rc<wgpu::Buffer>,
    parallelograms: Rc<wgpu::Buffer>,
    sdf: Rc<wgpu::Buffer>,
    triangles: Rc<wgpu::Buffer>,
    meshes: Rc<wgpu::Buffer>,
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

#[cfg(test)]
mod tests {
    use cgmath::EuclideanSpace;
    use crate::geometry::alias::Point;
    use super::*;

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
}