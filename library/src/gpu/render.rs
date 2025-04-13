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
use crate::serialization::filler::{floats_count, GpuFloatBufferFiller};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use bytemuck::checked::cast_slice;
use std::rc::Rc;
use wgpu::StoreOp;
use winit::dpi::PhysicalSize;

// TODO: work in progress: the whole file will be rewritten

pub(crate) struct Renderer {
    context: Rc<Context>,
    resources: Resources,
    buffers: Buffers,
    uniforms: Uniforms,
    pipeline_compute: ComputePipeline,
    pipeline_rasterization: RasterizationPipeline,
    shader_module: wgpu::ShaderModule,
    scene: Container,
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
        let compute = Self::create_compute_pipeline(&context, &resources, &buffers, &shader_module);
        let rasterization = Self::create_rasterization_pipeline(&context, &resources, &buffers, &shader_module);

        Ok(Self {
            context,
            resources,
            buffers,
            uniforms,
            pipeline_compute: compute,
            pipeline_rasterization: rasterization,
            shader_module,
            scene,
        })
    }

    fn serialize_uniforms(target: &Uniforms) -> [f32; Uniforms::SERIALIZED_SIZE_FLOATS] {
        let mut buffer = [0.0_f32; Uniforms::SERIALIZED_SIZE_FLOATS];
        target.serialize_into(&mut buffer);
        buffer
    }

    fn init_buffers(scene: &mut Container, uniforms: &Uniforms, resources: &Resources, frame_buffer_size: FrameBufferSize) -> Buffers {
        let empty_meshes_marker: Vec<f32> = vec![-1.0_f32; TriangleMesh::SERIALIZED_SIZE_FLOATS];
        let empty_triangles_marker: Vec<f32> = vec![-1.0_f32; Triangle::SERIALIZED_SIZE_FLOATS];
        let empty_bvh_marker: Vec<f32> = vec![-1.0_f32; BvhNode::SERIALIZED_SIZE_FLOATS];

        let serialized_triangles = scene.evaluate_serialized_triangles();
        let meshes = if serialized_triangles.empty()
        { &empty_meshes_marker } else { serialized_triangles.meshes() };
        let triangles = if serialized_triangles.empty()
        { &empty_triangles_marker } else { serialized_triangles.geometry() };
        let bvh = if serialized_triangles.empty()
        { &empty_bvh_marker } else { serialized_triangles.bvh() };

        let materials = if scene.materials_count() > 0
            { scene.evaluate_serialized_materials() } else { vec![0.0_f32; Material::SERIALIZED_SIZE_FLOATS] };

        let spheres = if scene.spheres_count() > 0
            { scene.evaluate_serialized_spheres() } else { vec![-1.0_f32; Sphere::SERIALIZED_SIZE_FLOATS] };

        let parallelograms = if scene.parallelograms_count() > 0
            { scene.evaluate_serialized_parallelograms() } else { vec![-1.0_f32; Parallelogram::SERIALIZED_SIZE_FLOATS] };

        let sdf = if scene.sdf_objects_count() > 0
            { scene.evaluate_serialized_sdf() } else { vec![-1.0_f32; SdfBox::SERIALIZED_SIZE_FLOATS] };

        // TODO: we can use inline defined shader data, rather than this complication
        let full_screen_quad_vertices: Vec<f32> = vec![-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0];

        Buffers {
            uniforms: resources.create_uniform_buffer("uniforms", cast_slice(&Self::serialize_uniforms(uniforms))),

            pixel_color_buffer: resources.create_pixel_color_buffer(frame_buffer_size),
            object_id_buffer: resources.create_object_id_buffer(frame_buffer_size),

            spheres: resources.create_storage_buffer_write_only("spheres", cast_slice(&spheres)),
            parallelograms: resources.create_storage_buffer_write_only("parallelograms", cast_slice(&parallelograms)),
            sdf: resources.create_storage_buffer_write_only("sdf", cast_slice(&sdf)),

            triangles: resources.create_storage_buffer_write_only("triangles from all meshes", cast_slice(triangles)),
            meshes: resources.create_storage_buffer_write_only("meshes meta data", cast_slice(meshes)),
            materials: resources.create_storage_buffer_write_only("materials", cast_slice(&materials)),
            bvh: resources.create_storage_buffer_write_only("bvh", cast_slice(bvh)),
            vertex: resources.create_vertex_buffer("full screen quad vertices", cast_slice(&full_screen_quad_vertices)),
        }
    }

    const FRAME_BUFFERS_GROUP_INDEX: u32 = 1;

    fn create_compute_pipeline<'a>(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> ComputePipeline {
        let mut compute_pipeline = ComputePipeline::new(resources.create_compute_pipeline(module));

        let uniforms_group_index =  0;
        let bind_group_layout = compute_pipeline.bind_group_layout(uniforms_group_index);
        let mut bind_group_builder = BindGroupBuilder::new(uniforms_group_index, Some("compute pipeline uniform group"), bind_group_layout);
            bind_group_builder
                .add_entry(0, &buffers.uniforms)
            ;
        compute_pipeline.commit_bind_group(context.device(), bind_group_builder);

        Self::create_frame_buffers_bindings_for_compute(context, buffers, &mut compute_pipeline);

        let scene_group_index =  2;
        let bind_group_layout = compute_pipeline.bind_group_layout(scene_group_index);
        let mut bind_group_builder = BindGroupBuilder::new(scene_group_index, Some("compute pipeline scene group"), bind_group_layout);
            bind_group_builder
                .add_entry(0, &buffers.spheres)
                .add_entry(1, &buffers.parallelograms)
                .add_entry(2, &buffers.sdf)
                .add_entry(3, &buffers.triangles)
                .add_entry(4, &buffers.meshes)
                .add_entry(5, &buffers.materials)
                .add_entry(6, &buffers.bvh)
            ;
        compute_pipeline.commit_bind_group(context.device(), bind_group_builder);

        compute_pipeline
    }

    fn create_frame_buffers_bindings_for_compute(context: &Context, buffers: &Buffers, compute_pipeline: &mut ComputePipeline) {
        let label = Some("compute pipeline frame buffers group");

        let bind_group_layout = compute_pipeline.bind_group_layout(Self::FRAME_BUFFERS_GROUP_INDEX);
        let mut bind_group_builder = BindGroupBuilder::new(Self::FRAME_BUFFERS_GROUP_INDEX, label, bind_group_layout);

        bind_group_builder
            .add_entry(0, &buffers.pixel_color_buffer)
            .add_entry(1, &buffers.object_id_buffer)
        ;

        compute_pipeline.commit_bind_group(context.device(), bind_group_builder);
    }

    fn create_rasterization_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> RasterizationPipeline {
        let mut rasterization_pipeline = RasterizationPipeline::new(resources.create_rasterization_pipeline(module));

        let uniforms_binding_index=  0;
        let bind_group_layout = rasterization_pipeline.bind_group_layout(uniforms_binding_index);
        let mut bind_group_builder = BindGroupBuilder::new(uniforms_binding_index, Some("rasterization pipeline uniform group"), bind_group_layout);
        bind_group_builder
            .add_entry(0, &buffers.uniforms)
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
            .add_entry(0, &buffers.pixel_color_buffer)
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

            Self::create_frame_buffers_bindings_for_compute(&self.context, &self.buffers, &mut self.pipeline_compute);
            Self::create_frame_buffers_bindings_for_rasterization(&self.context, &self.buffers, &mut self.pipeline_rasterization);
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
            let uniform_values = Self::serialize_uniforms(&self.uniforms);
            self.context.queue().write_buffer(&self.buffers.uniforms, 0, cast_slice(&uniform_values));

            self.uniforms.drop_reset_flag();
        }

        let work_groups_needed = self.uniforms.frame_buffer_size.area() / 64; // TODO: 64?
        self.compute_pass(&self.pipeline_compute, work_groups_needed);

        let view = &surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("rasterization pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // no need to clear as we will fill entire buffer during render
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };

        self.rasterization_pass(&mut render_pass_descriptor, &self.pipeline_rasterization, &self.buffers.vertex);
    }

    fn compute_pass(&self, compute_pipeline: &ComputePipeline, work_groups_needed: u32) {
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

    fn rasterization_pass(&self, rasterization_pass_descriptor: &mut wgpu::RenderPassDescriptor, rasterization_pipeline: &RasterizationPipeline, vertex_buffer: &wgpu::Buffer) {
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
    uniforms: wgpu::Buffer,

    pixel_color_buffer: wgpu::Buffer,
    object_id_buffer: wgpu::Buffer,

    spheres: wgpu::Buffer,
    parallelograms: wgpu::Buffer,
    sdf: wgpu::Buffer,
    triangles: wgpu::Buffer,
    meshes: wgpu::Buffer,
    materials: wgpu::Buffer,
    bvh: wgpu::Buffer,
    vertex: wgpu::Buffer,
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
}

impl SerializableForGpu for Uniforms {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Uniforms::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Uniforms::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;
        container.write_and_move_next(self.frame_buffer_size.width() as f64, &mut index);
        container.write_and_move_next(self.frame_buffer_size.height() as f64, &mut index);
        container.write_and_move_next(self.frame_number as f64, &mut index);
        container.write_and_move_next(if self.if_reset_framebuffer { 1.0 } else { 0.0 }, &mut index);

        self.camera.serialize_into(&mut container[index..]);
        index += Camera::SERIALIZED_SIZE_FLOATS;

        assert_eq!(index, Uniforms::SERIALIZED_SIZE_FLOATS);
    }
}