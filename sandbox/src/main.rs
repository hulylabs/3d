//#![deny(warnings)] TODO: switch on, when ready

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;
use winit::window::WindowId;
use winit::event_loop::ControlFlow;

use log::trace;
use log::info;
use log::error;

use library::Engine;

const WINDOW_TITLE: &str = "Rust Tracer Sandbox";

fn main() -> Result<(), String> {
    colog::init();

    let event_loop = EventLoop::new()
        .map_err(|e| format!(" event loop creation failed: {}", e))?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut application = Application::default();

    event_loop.run_app(&mut application)
        .map_err(|e| format!("event loop has failed: {}", e))?;

    Ok(())
}

#[derive(Default)]
struct Application {
    window: Option<Arc<Window>>,
    engine: Option<Engine>,
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

                match pollster::block_on(Engine::new(window.clone())) {
                    Ok(e) => {
                        self.engine = Some(e);
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
                self.engine.as_mut().map(|engine| {
                    engine.handle_window_resize(new_size);
                });
            }
            WindowEvent::ScaleFactorChanged { scale_factor: new_scale_factor, .. } => {
                info!("window scale factor changed to {:?}", new_scale_factor);
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().map(|window| {
                    self.engine.as_mut().map(|engine| {
                        engine.render(|| {
                            window.pre_present_notify();
                        });
                    });
                    window.request_redraw();
                });
            }
            _ => (),
        }
    }
}