//#![deny(warnings)] TODO: switch on, when ready

use std::env;
use std::path::Path;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::Window;
use winit::window::WindowId;
use winit::event_loop::ControlFlow;

use cgmath::{Matrix, Matrix4, SquareMatrix, Transform};

use log::trace;
use log::info;
use log::error;
use winit::keyboard::{Key, NamedKey};
use library::Engine;
use library::geometry::alias::{Point, Vector};
use library::geometry::transform::{Affine};
use library::objects::material::{Material, MaterialClass};
use library::scene::camera::Camera;
use library::scene::container::Container;

const WINDOW_TITLE: &str = "Rust Tracer Sandbox";

fn main() -> Result<(), String> {
    colog::init();

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

                let mut camera = Camera::new();
                camera.set(Some(Point::new(0.5, 0.0, 0.8)), Some(Point::new(0.5, 0.0, 0.0)), Some(Vector::new(0.0, 1.0, 0.0)));

                let mut scene = Container::new();

                let gold_metal = scene.add_material(&Material::new()
                    .with_class(MaterialClass::Mirror)
                    .with_albedo(1.0, 0.5, 0.0)
                    .with_specular_strength(0.00001)
                    .with_roughness(-1.0 / 4.0)
                    .with_refractive_index_eta(0.0));

                let blue_glass = scene.add_material(&Material::new()
                    .with_class(MaterialClass::Glass)
                    .with_albedo(0.0, 0.5, 0.9)
                    .with_refractive_index_eta(1.4));

                let purple_glass = scene.add_material(&Material::new()
                    .with_class(MaterialClass::Glass)
                    .with_albedo(1.0, 0.5, 0.9)
                    .with_refractive_index_eta(1.4));

                let green_mirror = scene.add_material(&Material::new()
                    .with_class(MaterialClass::Mirror)
                    .with_albedo(0.64, 0.77, 0.22)
                    .with_refractive_index_eta(1.4));

                let light_material = scene.add_material(&Material::new()
                    .with_emission(2.0, 2.0, 2.0));

                let black_material = scene.add_material(&Material::new()
                    .with_albedo(0.2, 0.2, 0.2)
                    .with_specular(0.2, 0.2, 0.2)
                    .with_specular_strength(0.05)
                    .with_roughness(0.95));

                let coral_material = scene.add_material(&Material::new()
                    .with_albedo(1.0, 0.5, 0.3)
                    .with_specular(0.2, 0.2, 0.2)
                    .with_specular_strength(0.01)
                    .with_roughness(0.0));

                let red_material = scene.add_material(&Material::new()
                    .with_albedo(0.75, 0.1, 0.1)
                    .with_specular(0.75, 0.1, 0.1)
                    .with_specular_strength(0.05)
                    .with_roughness(0.95));

                let green_material = scene.add_material(&Material::new()
                    .with_albedo(0.05, 0.55, 0.05)
                    .with_specular(0.05, 0.55, 0.05)
                    .with_specular_strength(0.05)
                    .with_roughness(0.95));

                scene.add_sphere(Point::new(1.5, 0.6, -1.0), 0.25, gold_metal);
                scene.add_sphere(Point::new(0.5, 0.0, -1.0), 0.25, blue_glass);
                scene.add_sphere(Point::new(1.5, 0.0, -1.0), 0.25, green_mirror);
                scene.add_sphere(Point::new(0.0, 0.0, -1.0), 0.25, coral_material);

                scene.add_quadrilateral(Point::new(-1.0, 1.0, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0), light_material);
                scene.add_quadrilateral(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), black_material);
                scene.add_quadrilateral(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), red_material);
                scene.add_quadrilateral(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), green_material);

                let large_box_material = scene.add_material(&Material::new()
                    .with_albedo(0.95, 0.95, 0.95)
                    .with_refractive_index_eta(2.5));
                let large_box_location =
                    Affine::from_translation(Vector::new(0.15, 0.6, -1.0)) * Affine::from_nonuniform_scale(3.65, 0.8, 0.25);
                let large_box = scene.add_mesh(Path::new("assets/cube.obj"), &large_box_location, large_box_material)
                    .inspect_err(|error| error!("failed to load cube from file: {}", error)); // TODO: handle file load failure

                {
                let box_location =
                    Affine::from_translation(Vector::new(-0.4, 0.1, -1.0)) * Affine::from_scale(0.4);
                let large_box = scene.add_mesh(Path::new("assets/cube.obj"), &box_location, gold_metal)
                    .inspect_err(|error| error!("failed to load cube from file: {}", error)); // TODO: handle file load failure
                }

                {
                let box_location =
                    Affine::from_translation(Vector::new(0.9, -0.4, -1.0)) * Affine::from_scale(0.4);
                let large_box = scene.add_mesh(Path::new("assets/cube.obj"), &box_location, purple_glass)
                    .inspect_err(|error| error!("failed to load cube from file: {}", error)); // TODO: handle file load failure
                }

                match pollster::block_on(Engine::new(window.clone(), scene, camera)) {
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
            WindowEvent::MouseWheel { delta: MouseScrollDelta::LineDelta(_, y), .. } => {
                self.engine.as_mut().map(|x| x.get_camera().zoom(y as f64));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                match event.logical_key {
                    Key::Named(NamedKey::ArrowUp) => {
                        self.engine.as_mut().map(|x| x.get_camera().move_up());
                    },
                    Key::Named(NamedKey::ArrowDown) => {
                        self.engine.as_mut().map(|x| x.get_camera().move_down());
                    },
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.engine.as_mut().map(|x| x.get_camera().move_left());
                    },
                    Key::Named(NamedKey::ArrowRight) => {
                        self.engine.as_mut().map(|x| x.get_camera().move_right());
                    },
                    _ => (),
                }
            }
            _ => (),
        }
    }
}