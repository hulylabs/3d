//#![deny(warnings)] TODO: switch on, when ready

pub mod geometry;
pub mod objects;
mod gpu;
pub mod scene;
mod serialization;
mod bvh;

use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winit::window::Window;
use log::info;
use thiserror::Error;
use crate::gpu::context::Context;
use crate::gpu::render::Renderer;
use crate::scene::camera::Camera;
use crate::scene::container::Container;

const DEVICE_LABEL: &str = "Rust Tracer Library";

pub struct Engine {
    /*Actually, we do not need any synchronization stuff; our code is
    single-threaded. But due to the design of the wgpu function, we
    are obliged to use thread-safe types to bypass compiler checks.*/
    device_was_lost: Arc<AtomicBool>,

    window_pixels_size: winit::dpi::PhysicalSize<u32>,

    context: Rc<Context>,

    window_output_surface: wgpu::Surface<'static>,
    window_surface_format: wgpu::TextureFormat,

    renderer: Renderer,
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
    #[error("internal error: {what:?}")]
    InternalError {
        what: String,
    },
}

impl Engine {
    pub async fn new(window: Arc<Window>, scene: Container, camera: Camera) -> Result<Engine, EngineInstantiationError> {
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

        let context = Rc::new(Context::new(graphics_device, commands_queue));
        let output_surface_format = surface_capabilities.formats[0];

        let renderer = Renderer::new(context.clone(), scene, camera, output_surface_format, window_pixels_size.width, window_pixels_size.height)
            .map_err(|e| EngineInstantiationError::InternalError {what: e.to_string()})?;

        let ware = Engine {
            device_was_lost: device_was_lost_flag.clone(),
            context: context.clone(),
            window_pixels_size,
            window_output_surface: window_surface,
            window_surface_format: output_surface_format,
            renderer,
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
        self.window_output_surface.configure(self.context.device(), &surface_config);
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

        self.renderer.execute(&surface_texture);

        pre_present_notify();
        surface_texture.present();
    }
}
