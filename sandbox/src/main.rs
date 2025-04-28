//#![deny(warnings)] TODO: switch on, when ready

mod sandbox;

use std::env;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;
use winit::window::WindowId;

use crate::sandbox::Sandbox;
use log::error;
use log::info;
use log::trace;
use library::Engine;

const WINDOW_TITLE: &str = "Rust Tracer Sandbox";

fn main() -> Result<(), String> {
    setup_logging();

    match env::current_dir() {
        Ok(path) => println!("current directory: {}", path.display()),
        Err(e) => eprintln!("error getting current directory: {}", e),
    }

    let event_loop = EventLoop::new()
        .map_err(|e| format!(" event loop creation failed: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut application = Application::default();

    event_loop.run_app(&mut application)
        .map_err(|e| format!("event loop has failed: {}", e))?;

    Ok(())
}

fn setup_logging() {
    let default_log_filter = format!("info,{}", Engine::get_reasonable_log_filter());
    let log_setup = env_logger::Env::default().default_filter_or(default_log_filter);
    env_logger::Builder::from_env(log_setup).init();
}

#[derive(Default)]
struct Application {
    window: Option<Arc<Window>>,
    demo: Option<Sandbox>,
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window_creation
            = event_loop.create_window(Window::default_attributes()
                .with_title(WINDOW_TITLE));

        match window_creation {
            Ok(ware) => {
                let window = Arc::new(ware);
                self.window = Some(window.clone());

                let demo = Sandbox::new(window.clone());
                match demo {
                    Ok(x) => {
                        self.demo = Some(x);
                    },
                    Err(error) => {
                        error!("failed to create an engine: {}", error);
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                error!("could not create the window: {}", error);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                trace!("exiting the loop via close request");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                info!("window resized to {:?}", new_size);
                self.demo.as_mut().map(|demo| {
                    demo.on_window_resized(new_size);
                });
            }
            WindowEvent::ScaleFactorChanged { scale_factor: new_scale_factor, .. } => {
                info!("window scale factor changed to {:?}", new_scale_factor);
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().map(|window| {
                    self.demo.as_mut().map(|demo| {
                        demo.on_redraw(window.clone());
                    });
                    window.request_redraw();
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.demo.as_mut().map(|demo| demo.on_mouse_button(state, button));
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.demo.as_mut().map(|demo| demo.on_mouse_move(position));
            }
            WindowEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(_, y), .. } => {
                self.demo.as_mut().map(|demo| demo.on_mouse_wheel(y as f64));
            }

            WindowEvent::KeyboardInput { event, .. } => {
                self.demo.as_mut().map(|demo| demo.on_keyboard_event(event));
            }
            _ => (),
        }
    }
}