use super::resources::Resources;
use crate::geometry::transform::Transformation;
use crate::gpu::context::Context;
use crate::scene::camera::Camera;
use crate::scene::container::Container;
use crate::serialization::helpers::{floats_count, GpuFloatBufferFiller};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use bytemuck::checked::cast_slice;
use std::rc::Rc;
use wgpu::{BindGroup, RenderPipeline, StoreOp};
use winit::dpi::PhysicalSize;
use crate::bvh::node::BvhNode;
use crate::objects::material::Material;
use crate::objects::parallelogram::Parallelogram;
use crate::objects::sphere::Sphere;
use crate::objects::triangle::Triangle;
use crate::objects::triangle_mesh::TriangleMesh;
// TODO: work in progress: the whole file will be rewritten

pub(crate) const CODE_FOR_GPU: &str = include_str!("../../assets/shaders/tracer.wgsl");

struct Buffers {
    uniforms: wgpu::Buffer,
    spheres: wgpu::Buffer,
    meshes: wgpu::Buffer,
    parallelograms: wgpu::Buffer,
    materials: wgpu::Buffer,
    transforms: wgpu::Buffer,
    bvh: wgpu::Buffer,
    triangles: wgpu::Buffer,
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
    camera: Camera,
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
            camera: camera.clone(),
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
            camera,
            scene,
        })
    }

    fn init_buffers(scene: &mut Container, uniforms: &Uniforms, resources: &Resources, frame_buffer_width: u32, frame_buffer_height: u32) -> Buffers {
        assert!(frame_buffer_width > 0);
        assert!(frame_buffer_height > 0);

        let uniform_values = {
            let mut buffer = [0.0_f32; Uniforms::SERIALIZED_SIZE_FLOATS];
            uniforms.serialize_into(&mut buffer);
            buffer
        };

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
            { scene.evaluate_serialized_materials() } else { vec![0.0; Material::SERIALIZED_SIZE_FLOATS] };

        let spheres = if scene.spheres_count() > 0
            { scene.evaluate_serialized_spheres() } else { vec![-1.0; Sphere::SERIALIZED_SIZE_FLOATS] };

        let parallelograms = if scene.parallelograms_count() > 0
            { scene.evaluate_serialized_parallelograms() } else { vec![-1.0; Parallelogram::SERIALIZED_SIZE_FLOATS] };

        // TODO: delete model transformations: looks like we can work in global coordinates
        let total_objects_count = scene.get_total_object_count();
        let mut transformations: Vec<f32> = vec![0.0_f32; total_objects_count * Transformation::SERIALIZED_SIZE_FLOATS];
        if total_objects_count <= 0 {
            transformations = vec![0.0_f32; Transformation::SERIALIZED_SIZE_FLOATS]
        } else {
            let identity = Transformation::identity();
            for i in 0..total_objects_count {
                identity.serialize_into(&mut transformations[(i * Transformation::SERIALIZED_SIZE_FLOATS)..])
            }
        }

        // TODO: we can use inline defined shader data, rather than this complication
        let full_screen_quad_vertices: Vec<f32> = vec![-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0];

        Buffers {
            uniforms: resources.create_uniform_buffer("uniforms", cast_slice(&uniform_values)),
            spheres: resources.create_storage_buffer_write_only("spheres", cast_slice(&spheres)),
            meshes: resources.create_storage_buffer_write_only("meshes meta data", cast_slice(meshes)),
            parallelograms: resources.create_storage_buffer_write_only("parallelograms", cast_slice(&parallelograms)),
            materials: resources.create_storage_buffer_write_only("materials", cast_slice(&materials)),
            transforms: resources.create_storage_buffer_write_only("transformations", cast_slice(&transformations)),
            bvh: resources.create_storage_buffer_write_only("bvh", cast_slice(bvh)),
            triangles: resources.create_storage_buffer_write_only("triangles from all meshes", cast_slice(triangles)),
            frame_buffer: resources.create_frame_buffer(frame_buffer_width, frame_buffer_height),
            vertex: resources.create_vertex_buffer("full screen quad vertices", cast_slice(&full_screen_quad_vertices)),
        }
    }

    fn create_compute_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> (wgpu::ComputePipeline, BindGroup) {
        let compute_pipeline = resources.create_compute_pipeline(module);

        let bind_group_compute = context.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bindGroup for work buffer"),
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
                    binding: 5,
                    resource: buffers.triangles.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: buffers.meshes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: buffers.transforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: buffers.materials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: buffers.bvh.as_entire_binding(),
                },
            ],
        });

        (compute_pipeline, bind_group_compute)
    }

    fn create_rasterization_pipeline(context: &Context, resources: &Resources, buffers: &Buffers, module: &wgpu::ShaderModule) -> (RenderPipeline, BindGroup) {
        let render_pipeline = resources.create_rasterization_pipeline(module);

        let bind_group = context.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render pipeline bind group"),
            layout: &render_pipeline.get_bind_group_layout(0),
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

        (render_pipeline, bind_group)
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

            if self.camera.moving() || self.camera.key_press() {
                self.uniforms.reset_frame_accumulation();
                self.uniforms.next_frame();
                self.camera.set_key_press(false);
            }
            self.uniforms.camera = self.camera.clone();

            {
                let mut uniform_array = [0.0_f32; Uniforms::SERIALIZED_SIZE_FLOATS];
                self.uniforms.serialize_into(&mut uniform_array);

                let bytes: &[u8] = cast_slice(&uniform_array);
                self.context.queue().write_buffer(&self.buffers.uniforms, 0, bytes); // TODO: rewrite with 'write_buffer_with'. May be we need kind of ping-pong or circular buffer here
            }

            self.uniforms.drop_reset_flag();
        }

        let work_groups_needed = (self.uniforms.out_frame_width * self.uniforms.out_frame_height) / 64; // TODO: 64?
        self.resources.compute_pass(&self.pipeline_compute, &self.bind_group_compute, work_groups_needed);

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
            timestamp_writes: None, // TODO: what is this?
        };

        self.resources
            .render_pass(&mut render_pass_descriptor, &self.pipeline_rasterization, &self.bind_group_rasterization, &self.buffers.vertex);
    }

    #[must_use]
    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera
    }
}
