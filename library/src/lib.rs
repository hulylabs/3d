//#![deny(warnings)]

#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::bool_comparison)]
#![allow(clippy::needless_range_loop)]

pub mod geometry;
pub mod objects;
pub mod scene;
pub mod utils;
#[cfg(feature = "denoiser")]
mod denoiser;
mod bvh;
mod serialization;
mod gpu;
mod tests;
pub mod sdf;

use crate::gpu::context::Context;
use crate::gpu::frame_buffer_size::FrameBufferSize;
use crate::gpu::render::Renderer;
use crate::scene::camera::Camera;
use crate::scene::container::Container;
use crate::utils::min_max_time_measurer::MinMaxTimeMeasurer;
use crate::utils::object_uid::ObjectUid;
use crate::utils::sliding_time_frame::SlidingTimeFrame;
use crate::utils::time_throttled_logger::TimeThrottledInfoLogger;
use log::info;
use std::cmp::max;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use wgpu::{Adapter, Trace};
use winit::window::Window;
use crate::gpu::color_buffer_evaluation::RenderStrategyId;

const DEVICE_LABEL: &str = "Rust Tracer Library";

const FPS_MEASUREMENT_SAMPLES: usize = 15;
const FPS_WRITE_INTERVAL: Duration = Duration::from_secs(2);

#[cfg(feature = "denoiser")]
pub const RAYS_ACCUMULATIONS_PER_FRAME: usize = 10;
#[cfg(not(feature = "denoiser"))]
pub const RAYS_ACCUMULATIONS_PER_FRAME: usize = 1;

const PIXEL_SUBDIVISION_MONTE_CARLO: u32 = 2;
const PIXEL_SUBDIVISION_DETERMINISTIC: u32 = 4;

pub struct Engine {
    /*Actually, we do not need any synchronization stuff; our code is
    single-threaded. But due to the design of the wgpu function, we
    are obliged to use thread-safe types to bypass compiler checks.*/
    device_was_lost: Arc<AtomicBool>,

    window_pixels_size: winit::dpi::PhysicalSize<u32>,
    ignore_render_requests: bool,

    context: Rc<Context>,

    window_output_surface: wgpu::Surface<'static>, // TODO: actually this object is not quite 'static; in fact here we do not know anything about that, how static it is
    window_surface_format: wgpu::TextureFormat,

    renderer: Renderer,
    
    fps_measurer: SlidingTimeFrame,
    denoising_measurer: MinMaxTimeMeasurer,
    performance_reporter: TimeThrottledInfoLogger,
}

#[derive(Error, Debug)]
pub enum EngineInstantiationError {
    #[error("failed to create window surface: {what:?}")]
    SurfaceCreationError {
        what: String,
    },
    #[error("failed to request adapter: {what:?}")]
    AdapterRequisitionError{
        what: String,
    },
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
    #[must_use]
    pub fn get_reasonable_log_filter() -> &'static str {
        "wgpu=warn,naga=warn"
    }
    
    pub async fn new(window: Arc<Window>, scene: Container, camera: Camera) -> Result<Engine, EngineInstantiationError> {
        let wgpu_instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::empty(),
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
            .map_err(|error| EngineInstantiationError::AdapterRequisitionError{what: error.to_string()})?;

        log_adapter_info(&graphics_adapter);
        
        let (graphics_device, commands_queue) = graphics_adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some(DEVICE_LABEL),
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: Trace::Off,
            })
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
                info!("device was lost: {:?}, {}", reason, message);
                device_was_lost.store(true, Ordering::SeqCst);
            }
        };
        graphics_device.set_device_lost_callback(lost_device_handler);

        let context = Rc::new(Context::new(graphics_device, commands_queue));
        let output_surface_format = surface_capabilities.formats[0];

        let frame_buffer_size = FrameBufferSize::new(max(1, window_pixels_size.width), max(1, window_pixels_size.height));
        let renderer 
            = Renderer::new(
                context.clone(), 
                scene, 
                camera, 
                output_surface_format, 
                frame_buffer_size, 
                RenderStrategyId::Deterministic, 
                PIXEL_SUBDIVISION_DETERMINISTIC,
            )
            .map_err(|e| EngineInstantiationError::InternalError {what: e.to_string()})?;

        let ware = Engine {
            device_was_lost: device_was_lost_flag.clone(),
            context: context.clone(),
            window_pixels_size,
            ignore_render_requests: false,
            window_output_surface: window_surface,
            window_surface_format: output_surface_format,
            renderer,

            fps_measurer: SlidingTimeFrame::new(FPS_MEASUREMENT_SAMPLES),
            denoising_measurer: MinMaxTimeMeasurer::default(),
            performance_reporter: TimeThrottledInfoLogger::new(FPS_WRITE_INTERVAL),
        };

        ware.configure_surface();

        Ok(ware)
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.window_surface_format,
            view_formats: vec![self.window_surface_format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.window_pixels_size.width,
            height: self.window_pixels_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 1,
        };

        self.window_output_surface.configure(self.context.device(), &surface_config);
    }

    fn configure_render(&mut self) {
        self.renderer.set_output_size(self.window_pixels_size);
        self.fps_measurer.start();
    }

    // TODO: add handling of window obscuring → request to unload all occupied resources (iOS)

    pub fn handle_window_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            info!("window resized to zero — will not respond to render requests");
            self.ignore_render_requests = true;
            return;
        }

        if self.ignore_render_requests {
            info!("window resized — will respond to render requests");
            self.ignore_render_requests = false;
        }

        if new_size == self.window_pixels_size {
            return;
        }
        self.window_pixels_size = new_size;
        self.configure_surface();
        self.configure_render();
    }

    pub fn render<Code: Fn()>(&mut self, pre_present_notify: Code) {
        if self.ignore_render_requests {
            return;
        }

        if self.device_was_lost.load(Ordering::SeqCst) {
            // TODO: handle lost device
        }

        let surface_texture = self
            .window_output_surface
            .get_current_texture()
            .expect("failed to acquire next image in the swapchain");

        if surface_texture.suboptimal {
            // TODO: schedule surface reconfigure?
        }

        if self.renderer.is_monte_carlo() {
            for _ in 0..RAYS_ACCUMULATIONS_PER_FRAME {
                self.renderer.accumulate_more_rays();
            }   
        } else {
            self.renderer.accumulate_more_rays();
        } 

        #[cfg(feature = "denoiser")]
        {
            if self.renderer.is_monte_carlo() {
                self.denoising_measurer.start();
                self.renderer.denoise_accumulated_image();
                self.denoising_measurer.stop();   
            }
        }

        self.renderer.present(&surface_texture);

        pre_present_notify();
        surface_texture.present();

        self.fps_measurer.sample();

        self.write_performance_report();
    }

    fn write_performance_report(&mut self) {
        let average_frame_time = self.fps_measurer.average_delta();
        let fps = 1.0 / average_frame_time.as_secs_f32();

        let performance_report = 
            if cfg!(feature = "denoiser") {
                format!(
                    "CPU observed FPS: {}; Denoising (ms): min={}, max={}, current={}",
                    fps,
                    self.denoising_measurer.min_time().as_millis(),
                    self.denoising_measurer.max_time().as_millis(),
                    self.denoising_measurer.last_time().as_millis(),
                )
            } else {
                format!("CPU observed FPS: {}", fps)
            };
        
        self.performance_reporter.do_write(performance_report);
    }

    #[must_use]
    pub fn object_in_pixel(&self, x: u32, y: u32) -> Option<ObjectUid> {
        assert!(x < self.window_pixels_size.width);
        assert!(y < self.window_pixels_size.height);
        self.renderer.object_in_pixel(x, y)
    }

    #[must_use]
    pub fn camera(&mut self) -> &mut Camera {
        self.renderer.camera()
    }

    #[must_use]
    pub fn scene(&mut self) -> &mut Container {
        self.renderer.scene()
    }
    
    pub fn use_monte_carlo_render(&mut self) {
        self.renderer.set_render_strategy(RenderStrategyId::MonteCarlo, PIXEL_SUBDIVISION_MONTE_CARLO);
    }
    
    pub fn use_deterministic_render(&mut self) {
        self.renderer.set_render_strategy(RenderStrategyId::Deterministic, PIXEL_SUBDIVISION_DETERMINISTIC);
    }
}

fn log_adapter_info(adapter: &Adapter) {
    let adapter_info = adapter.get_info();
    info!(
        "Adapter Info:\n\
         Name: {}\n\
         Backend: {:?}\n\
         Vendor: {:#x}\n\
         Device: {:#x}\n\
         Device Type: {:?}\n\
         Driver: {:?}\n\
         Driver Info: {:?}",
        adapter_info.name,
        adapter_info.backend,
        adapter_info.vendor,
        adapter_info.device,
        adapter_info.device_type,
        adapter_info.driver,
        adapter_info.driver_info,
    );
}
