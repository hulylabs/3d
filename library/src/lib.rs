//#![deny(warnings)] TODO: switch on, when ready

mod geometry;
mod objects;
mod utils;
mod gpu;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::window::Window;
use log::info;
use thiserror::Error;

const DEVICE_LABEL: &str = "Rust Tracer Library";

pub struct Engine {
    /*Actually, we do not need any synchronization stuff; our code is
    single-threaded. But due to the design of the wgpu function, we
    are obliged to use thread-safe types to bypass compiler checks.*/
    device_was_lost: Arc<AtomicBool>,

    window_pixels_size: winit::dpi::PhysicalSize<u32>,

    graphics_device: wgpu::Device,
    commands_queue: wgpu::Queue,

    window_output_surface: wgpu::Surface<'static>,
    window_surface_format: wgpu::TextureFormat,
}

#[derive(Error, Debug)]
pub enum EngineInstantiationError {
    #[error("failed to create window surface: {what:?}")]
    SurfaceCreationError {
        what: String,
    },
    #[error("failed to request adapter")]
    AdapterRequisitionError
    ,
    #[error("failed to select device: {what:?}")]
    DeviceSelectionError {
        what: String,
    },
    #[error("surface is incompatible with the device")]
    SurfaceCompatibilityError
    ,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Result<Engine, EngineInstantiationError> {
        let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let window_pixels_size = window.inner_size();
        let window_surface = wgpu_instance.create_surface(window.clone())
            .map_err(|e| EngineInstantiationError::SurfaceCreationError{what: e.to_string()})?;

        let graphics_adapter = wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&window_surface),
                ..Default::default()
            })
            .await
            .ok_or(EngineInstantiationError::AdapterRequisitionError)?;

        let (graphics_device, commands_queue) = graphics_adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(DEVICE_LABEL),
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            }, Some(Path::new("./")))
            .await
            .map_err(|e| EngineInstantiationError::DeviceSelectionError {what: e.to_string()})?;

        let surface_capabilities = window_surface.get_capabilities(&graphics_adapter);
        if surface_capabilities.formats.is_empty() {
            return Err(EngineInstantiationError::SurfaceCompatibilityError);
        }

        let device_was_lost_flag = Arc::new(AtomicBool::new(false));

        let lost_device_handler = {
            let device_was_lost = Arc::clone(&device_was_lost_flag);
            move |reason, message| {
                info!("device was lost: {}, {}", format!("{:?}", reason), message);
                device_was_lost.store(true, Ordering::SeqCst);
            }
        };
        graphics_device.set_device_lost_callback(lost_device_handler);

        let ware = Engine {
            device_was_lost: device_was_lost_flag.clone(),
            graphics_device,
            commands_queue,
            window_pixels_size,
            window_output_surface: window_surface,
            window_surface_format: surface_capabilities.formats[0],
        };

        ware.configure_surface();

        Ok(ware)
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.window_surface_format,
            view_formats: vec![self.window_surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.window_pixels_size.width,
            height: self.window_pixels_size.height,
            desired_maximum_frame_latency: 0,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.window_output_surface.configure(&self.graphics_device, &surface_config);
    }

    pub fn handle_window_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.window_pixels_size = new_size;
        self.configure_surface();
    }

    pub fn render<Code>(&mut self, pre_present_notify: Code) where Code : Fn() {
        if self.device_was_lost.load(Ordering::SeqCst) {
            // TODO: handle lost device
        }

        let surface_texture = self
            .window_output_surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // without add_srgb_suffix() the image we will be working with might not be "gamma correct" TODO: <- do we need this
                format: Some(self.window_surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self.graphics_device.create_command_encoder(&Default::default());

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // other drawing commands go here

        drop(render_pass);
        let command_buffer = encoder.finish();
        self.commands_queue.submit([command_buffer]);

        pre_present_notify();
        surface_texture.present();
    }
}
