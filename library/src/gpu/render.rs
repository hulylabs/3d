use super::resources::Resources;
use crate::bvh::node::BvhNode;
use crate::gpu::context::Context;
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
use wgpu::{BindGroup, RenderPipeline, StoreOp};
use winit::dpi::PhysicalSize;

// TODO: work in progress: the whole file will be rewritten

pub struct Renderer {
    context: Rc<Context>,
    resources: Resources,
    buffers: Buffers,
    uniforms: Uniforms,
    pipeline_compute: wgpu::ComputePipeline,
    pipeline_rasterization: RenderPipeline,
    bind_group_compute: BindGroup,
    bind_group_rasterization: BindGroup,
    shader_module: wgpu::ShaderModule,
    scene: Container,
}

impl Renderer {
    pub fn new(
        context: Rc<Context>,
        scene_container: Container,
        camera: Camera,
        presentation_format: wgpu::TextureFormat,
        out_frame_width: u32,
        out_frame_height: u32)
    -> anyhow::Result<Self> {
        assert!(out_frame_width > 0);
        assert!(out_frame_height > 0);
        let uniforms = Uniforms {
            out_frame_width,
            out_frame_height,
            frame_number: 0,
            if_reset_framebuffer: false,
            camera,
        };

        let mut scene = scene_container;

        let resources = Resources::new(context.clone(), presentation_format);
        let shader_module = resources.create_shader_module("tracer shader", CODE_FOR_GPU)?;
        let buffers = Self::init_buffers(&mut scene, &uniforms, &resources, out_frame_width, out_frame_height);
        let compute = Self::create_compute_pipeline(&context, &resources, &buffers, &shader_module);
        let rasterization = Self::create_rasterization_pipeline(&context, &resources, &buffers, &shader_module);

        Ok(Self {
            context,
            resources,
            buffers,
            uniforms,
            pipeline_compute: compute.0,
            pipeline_rasterization: rasterization.0,
            bind_group_compute: compute.1,
            bind_group_rasterization: rasterization.1,
            shader_module,
            scene,
        })
    }

    fn serialize_uniforms(target: &Uniforms) -> [f32; Uniforms::SERIALIZED_SIZE_FLOATS] {
        let mut buffer = [0.0_f32; Uniforms::SERIALIZED_SIZE_FLOATS];
        target.serialize_into(&mut buffer);
        buffer
    }

    fn init_buffers(scene: &mut Container, uniforms: &Uniforms, resources: &Resources, frame_buffer_width: u32, frame_buffer_height: u32) -> Buffers {
        assert!(frame_buffer_width > 0);
        assert!(frame_buffer_height > 0);

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
            spheres: resources.create_storage_buffer_write_only("spheres", cast_slice(&spheres)),
            meshes: resources.create_storage_buffer_write_only("meshes meta data", cast_slice(meshes)),
            parallelograms: resources.create_storage_buffer_write_only("parallelograms", cast_slice(&parallelograms)),
            materials: resources.create_storage_buffer_write_only("materials", cast_slice(&materials)),
            bvh: resources.create_storage_buffer_write_only("bvh", cast_slice(bvh)),
            triangles: resources.create_storage_buffer_write_only("triangles from all meshes", cast_slice(triangles)),
            sdf: resources.create_storage_buffer_write_only("sdf", cast_slice(&sdf)),
            frame_buffer: resources.create_frame_buffer(frame_buffer_width, frame_buffer_height),
            vertex: resources.create_vertex_buffer("full screen quad vertices", cast_slice(&full_screen_quad_vertices)),
        }
    }

    fn create_compute_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> (wgpu::ComputePipeline, BindGroup) {
        let compute_pipeline = resources.create_compute_pipeline(module);

        let bind_group_compute = context.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute pipeline bind group"),
            layout: &compute_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffers.spheres.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.parallelograms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.frame_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buffers.sdf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: buffers.triangles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: buffers.meshes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: buffers.materials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: buffers.bvh.as_entire_binding(),
                },
            ],
        });

        (compute_pipeline, bind_group_compute)
    }

    fn create_rasterization_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> (RenderPipeline, BindGroup) {
        let rasterization_pipeline = resources.create_rasterization_pipeline(module);

        let bind_group = context.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rasterization pipeline bind group"),
            layout: &rasterization_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffers.uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: buffers.frame_buffer.as_entire_binding(),
                },
            ],
        });

        (rasterization_pipeline, bind_group)
    }

    pub(crate) fn set_output_size(&mut self, new_size: PhysicalSize<u32>) {
        let previous_frame_size = self.uniforms.frame_size();
        self.uniforms.set_frame_size(new_size);
        let new_frame_size = self.uniforms.frame_size();

        if previous_frame_size < new_frame_size {
            let frame_buffer = self.resources.create_frame_buffer(self.uniforms.out_frame_width, self.uniforms.out_frame_height);

            self.buffers.frame_buffer = frame_buffer;

            let compute = Self::create_compute_pipeline(&self.context, &self.resources, &self.buffers, &self.shader_module);
            let rasterization = Self::create_rasterization_pipeline(&self.context, &self.resources, &self.buffers, &self.shader_module);

            self.pipeline_compute = compute.0;
            self.pipeline_rasterization = rasterization.0;
            self.bind_group_compute = compute.1;
            self.bind_group_rasterization = rasterization.1;
        }
    }

    pub fn execute(&mut self, surface_texture: &wgpu::SurfaceTexture)  {
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

        let work_groups_needed = (self.uniforms.out_frame_width * self.uniforms.out_frame_height) / 64; // TODO: 64?
        self.compute_pass(&self.pipeline_compute, &self.bind_group_compute, work_groups_needed);

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

        self.rasterization_pass(&mut render_pass_descriptor, &self.pipeline_rasterization, &self.bind_group_rasterization, &self.buffers.vertex);
    }

    fn compute_pass(&self, compute_pipeline: &wgpu::ComputePipeline, bind_group_compute: &BindGroup, work_groups_needed: u32) {
        let mut encoder = self.create_command_encoder("compute pass encoder");
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor
            {
                label: Some("ray tracing compute pass"),
                timestamp_writes: None, // TODO: what can be used for?
            });

            pass.set_pipeline(compute_pipeline);
            pass.set_bind_group(0, bind_group_compute, &[]);
            pass.dispatch_workgroups(work_groups_needed, 1, 1);
        }
        let command_buffer = encoder.finish();
        self.context.queue().submit(Some(command_buffer));
    }

    fn rasterization_pass(&self, rasterization_pass_descriptor: &mut wgpu::RenderPassDescriptor, rasterization_pipeline: &RenderPipeline, bind_group: &BindGroup, vertex_buffer: &wgpu::Buffer) {
        let mut encoder = self.create_command_encoder("rasterization pass encoder");
        {
            let mut rasterization_pass = encoder.begin_render_pass(rasterization_pass_descriptor);
            rasterization_pass.set_pipeline(rasterization_pipeline);
            rasterization_pass.set_bind_group(0, bind_group, &[]);
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
    spheres: wgpu::Buffer,
    meshes: wgpu::Buffer,
    parallelograms: wgpu::Buffer,
    materials: wgpu::Buffer,
    bvh: wgpu::Buffer,
    triangles: wgpu::Buffer,
    sdf: wgpu::Buffer,
    frame_buffer: wgpu::Buffer,
    vertex: wgpu::Buffer,
}

struct Uniforms {
    out_frame_width: u32,
    out_frame_height: u32,
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
        self.out_frame_width = new_size.width;
        self.out_frame_height = new_size.height;
        self.reset_frame_accumulation();
    }

    fn next_frame(&mut self) {
        self.frame_number += 1;
    }

    #[must_use]
    fn frame_size(&self) -> u32 {
        self.out_frame_width * self.out_frame_height
    }

    const SERIALIZED_QUARTET_COUNT: usize = 1 + Camera::SERIALIZED_QUARTET_COUNT;
}

impl SerializableForGpu for Uniforms {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Uniforms::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Uniforms::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;
        container.write_and_move_next(self.out_frame_width as f64, &mut index);
        container.write_and_move_next(self.out_frame_height as f64, &mut index);
        container.write_and_move_next(self.frame_number as f64, &mut index);
        container.write_and_move_next(if self.if_reset_framebuffer { 1.0 } else { 0.0 }, &mut index);

        self.camera.serialize_into(&mut container[index..]);
        index += Camera::SERIALIZED_SIZE_FLOATS;

        assert_eq!(index, Uniforms::SERIALIZED_SIZE_FLOATS);
    }
}