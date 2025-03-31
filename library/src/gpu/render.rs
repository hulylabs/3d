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

// TODO: work in progress: the whole file will be rewritten

pub(crate) const CODE_FOR_GPU: &str = include_str!("../../assets/shaders/tracer.wgsl");

struct Buffers {
    uniforms: wgpu::Buffer,
    spheres: wgpu::Buffer,
    meshes: wgpu::Buffer,
    quads: wgpu::Buffer,
    materials: wgpu::Buffer,
    transforms: wgpu::Buffer,
    bvh: wgpu::Buffer,
    triangles: wgpu::Buffer,
    frame_buffer: wgpu::Buffer,
    vertex: wgpu::Buffer,
}

struct Uniforms {
    screen_size_width: f32,
    screen_size_height: f32,
    frame_number: u32,
    if_reset_framebuffer: bool,
    camera: Camera,
}

impl Uniforms {
    const SERIALIZED_QUARTET_COUNT: usize = 1 + Camera::SERIALIZED_QUARTET_COUNT;
}

impl SerializableForGpu for Uniforms {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Uniforms::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Uniforms::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;
        container.write_and_move_next(self.screen_size_width, &mut index);
        container.write_and_move_next(self.screen_size_height, &mut index);
        container.write_and_move_next(self.frame_number as f32, &mut index);
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
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: RenderPipeline,
    bind_group_compute: BindGroup,
    bind_group: BindGroup,
    frame_number: u32,
    camera: Camera,
    scene: Container,
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(
        context: Rc<Context>,
        scene_container: Container,
        camera: Camera,
        presentation_format: wgpu::TextureFormat, // TODO: can we get this form 'frame_buffer'?
        frame_width: u32,
        frame_height: u32) -> anyhow::Result<Self> {
        let uniforms = Uniforms {
            screen_size_width: frame_width as f32,
            screen_size_height: frame_height as f32,
            frame_number: 0,
            if_reset_framebuffer: false,
            camera: camera.clone(),
        };

        let mut scene = scene_container;

        let resources = Resources::new(context.clone(), presentation_format);
        let shader_module = resources.create_shader_module("tracer shader", CODE_FOR_GPU)?;
        let buffers = Self::init_buffers(&mut scene, &uniforms, &resources, frame_width, frame_height);
        let compute = Self::create_compute_pipeline(&context, &resources, &buffers, &shader_module);
        let render = Self::create_render_pipeline(&resources, &context, &buffers, &shader_module);

        Ok(Self {
            context,
            resources,
            buffers,
            uniforms,
            compute_pipeline: compute.0,
            render_pipeline: render.0,
            bind_group_compute: compute.1,
            bind_group: render.1,
            frame_number: 0,
            camera,
            scene,
            width: frame_width,
            height: frame_height,
        })
    }

    fn init_buffers(scene: &mut Container, uniforms: &Uniforms, resources: &Resources, width: u32, height: u32) -> Buffers {
        let mut uniform_array = [0.0; Uniforms::SERIALIZED_SIZE_FLOATS];
        uniforms.serialize_into(&mut uniform_array);

        let meshes = scene.evaluate_serialized_meshes();
        let spheres = scene.evaluate_serialized_spheres();
        let quadrilaterals = scene.evaluate_serialized_quadrilaterals();
        let materials = scene.evaluate_serialized_materials();
        let triangles = scene.evaluate_serialized_triangles(); // TODO: wgpu fails if we've got zero triangles, we need to handle that

        // TODO: delete model transformations: looks like we can work in global coordinates
        let total_objects_count = scene.get_total_object_count();
        let mut transformations = vec![0.0_f32; total_objects_count * Transformation::SERIALIZED_SIZE_FLOATS];
        let identity = Transformation::identity();
        for i in 0..total_objects_count {
            identity.serialize_into(&mut transformations[(i * Transformation::SERIALIZED_SIZE_FLOATS)..])
        }

        let bvh = scene.evaluate_serialized_bvh();

        let frame_buffer: Vec<f32> = vec![0.0; (width * height * 4) as usize];
        let vertex_data: Vec<f32> = vec![-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 1.0];

        Buffers {
            uniforms: resources.create_uniform_buffer("uniforms", cast_slice(&uniform_array)),
            spheres: resources.create_storage_buffer_write_only("serialized spheres", cast_slice(&spheres)),
            meshes: resources.create_storage_buffer_write_only("serialized meshes", cast_slice(&meshes)),
            quads: resources.create_storage_buffer_write_only("serialized quadriliterals", cast_slice(&quadrilaterals)),
            materials: resources.create_storage_buffer_write_only("serialized materials", cast_slice(&materials)),
            transforms: resources.create_storage_buffer_write_only("transformations", cast_slice(&transformations)),
            bvh: resources.create_storage_buffer_write_only("serialized bvh", cast_slice(&bvh)),
            triangles: resources.create_storage_buffer_write_only("triangles from all meshes", cast_slice(&triangles)),
            frame_buffer: resources.create_storage_buffer_read_write("frame buffer", cast_slice(&frame_buffer)),
            vertex: resources.create_vertex_buffer("full screen quad vertices", cast_slice(&vertex_data)),
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
                    resource: buffers.quads.as_entire_binding(),
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

    fn create_render_pipeline(resources: &Resources, context: &Context, buffers: &Buffers, module: &wgpu::ShaderModule) -> (RenderPipeline, BindGroup) {
        let render_pipeline = resources.create_render_pipeline(module);

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

    pub fn execute(&mut self, surface_texture: &wgpu::SurfaceTexture)  {
        {
            self.frame_number += 1;

            self.uniforms.frame_number = self.frame_number;
            self.uniforms.if_reset_framebuffer = self.camera.moving() || self.camera.key_press();

            if self.camera.moving() || self.camera.key_press() {
                self.frame_number = 1;
                self.camera.set_key_press(false);
            }
            self.uniforms.camera = self.camera.clone();

            {
                let mut uniform_array = [0.0_f32; Uniforms::SERIALIZED_SIZE_FLOATS];
                self.uniforms.serialize_into(&mut uniform_array);

                let bytes: &[u8] = cast_slice(&uniform_array);
                self.context.queue().write_buffer(&self.buffers.uniforms, 0, bytes);
            }
        }

        let work_groups_needed = (self.width * self.height) / 64;
        self.resources.compute_pass(&self.compute_pipeline, &self.bind_group_compute, work_groups_needed);

        let view = &surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("renderPass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None, // TODO: what is this?
        };

        self.resources
            .render_pass(&mut render_pass_descriptor, &self.render_pipeline, &self.bind_group, &self.buffers.vertex);
    }
}
