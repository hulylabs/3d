use std::env;
use cgmath::Deg;
use library::geometry::alias::{Point, Vector};
use library::geometry::transform::{Affine, Transformation};
use library::objects::material::{Material, MaterialClass};
use library::scene::camera::{Camera, OrthographicCamera, PerspectiveCamera};
use library::scene::container::Container;
use library::scene::mesh_warehouse::MeshWarehouse;
use library::Engine;
use log::error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use library::objects::material_index::MaterialIndex;
use library::utils::object_uid::ObjectUid;

#[must_use]
fn make_default_camera() -> Camera {
    let mut camera = Camera::new_perspective_camera(0.8, Point::new(0.0, 0.0, 0.0));
    camera.move_horizontally(0.5);

    camera.set_zoom_speed(-0.3);
    camera.set_linear_speed(0.1);
    camera.set_rotation_speed(Deg(-0.1));

    camera
}

#[derive(Copy, Clone)]
struct SelectedObject {
    material: MaterialIndex,
    uid: ObjectUid,
}

pub(super) struct Sandbox {
    engine: Engine,

    left_mouse_down: bool,
    last_cursor_position: Option<(f64, f64)>,

    selected_object: Option<SelectedObject>,
    selected_object_material: MaterialIndex,
}

impl Sandbox {
    pub(super) fn on_window_resized(&mut self, new_size: PhysicalSize<u32>) {
        self.engine.handle_window_resize(new_size);
    }

    pub(super) fn on_redraw(&mut self, window: Arc<Window>) {
        self.engine.render(|| {
            window.pre_present_notify();
        });
    }
    
    pub(super) fn on_mouse_move(&mut self, position: PhysicalPosition<f64>) {
        let (current_x, current_y) = (position.x, position.y);
        if self.left_mouse_down {
            if let Some((last_x, _last_y)) = self.last_cursor_position {
                let delta_x = current_x - last_x;
                self.engine.camera().rotate_horizontal(delta_x);
            }
        }
        self.last_cursor_position = Some((current_x, current_y));
    }

    pub(super) fn on_mouse_button(&mut self, state: ElementState, button: MouseButton) {
        if MouseButton::Right == button {
            if let Some((last_x, last_y)) = self.last_cursor_position {
                let clicked_object_or_none = self.engine.object_in_pixel(last_x as u32, last_y as u32);
                let scene = self.engine.scene();
                
                if let Some(selected_object) = self.selected_object {
                    scene.set_material(selected_object.uid, selected_object.material);
                }
                
                if let Some(clicked_object) = clicked_object_or_none {
                    let old_material = scene.material_of(clicked_object);
                    scene.set_material(clicked_object, self.selected_object_material);
                    self.selected_object = Some(SelectedObject{material: old_material, uid: clicked_object});
                } else {
                    self.selected_object = None;
                }
            }
        } else if MouseButton::Left == button {
            self.left_mouse_down = state == ElementState::Pressed;
        } else if MouseButton::Middle == button {
            if state == ElementState::Pressed {
                if let Some((last_x, last_y)) = self.last_cursor_position {
                    let clicked_object_or_none = self.engine.object_in_pixel(last_x as u32, last_y as u32);
                    if let Some(clicked_object) = clicked_object_or_none {
                        self.engine.scene().delete(clicked_object);

                        if let Some(selected_object) = self.selected_object {
                            if selected_object.uid == clicked_object {
                                self.selected_object = None;
                            }
                        }
                    }
                }   
            }
        }
    }
    
    pub(super) fn on_mouse_wheel(&mut self, delta: f64) {
        self.engine.camera().zoom(delta);
    }
    
    pub(super) fn on_keyboard_event(&mut self, event: KeyEvent) {
        match event.logical_key {
            Key::Named(NamedKey::ArrowUp) => {
                self.engine.camera().move_vertically(1.0);
            },
            Key::Named(NamedKey::ArrowDown) => {
                self.engine.camera().move_vertically(-1.0);
            },
            Key::Named(NamedKey::ArrowRight) => {
                self.engine.camera().move_horizontally(1.0);
            },
            Key::Named(NamedKey::ArrowLeft) => {
                self.engine.camera().move_horizontally(-1.0);
            },
            Key::Character(letter_key) => {
                if "p" == letter_key {
                    self.engine.camera().set_kind(Box::new(PerspectiveCamera {}));
                } else if "o" == letter_key {
                    self.engine.camera().set_kind(Box::new(OrthographicCamera {}));
                } else if "r" == letter_key {
                    self.engine.camera().set_from(&make_default_camera());
                }
            }
            _ => (),
        }
    }
    
    pub(super) fn new(window: Arc<Window>) -> anyhow::Result<Self> {

        let camera = make_default_camera();
        
        let mut scene = Container::new();

        let gold_metal = scene.materials().add(&Material::new()
            .with_class(MaterialClass::Mirror)
            .with_albedo(1.0, 0.5, 0.0)
            .with_specular_strength(0.00001)
            .with_roughness(-1.0 / 4.0)
            .with_refractive_index_eta(0.0));

        let blue_glass = scene.materials().add(&Material::new()
            .with_class(MaterialClass::Glass)
            .with_albedo(0.0, 0.5, 0.9)
            .with_refractive_index_eta(1.4));

        let purple_glass = scene.materials().add(&Material::new()
            .with_class(MaterialClass::Glass)
            .with_albedo(1.0, 0.5, 0.9)
            .with_refractive_index_eta(1.4));

        let red_glass = scene.materials().add(&Material::new()
            .with_class(MaterialClass::Glass)
            .with_albedo(1.0, 0.2, 0.0)
            .with_refractive_index_eta(1.4));

        let green_mirror = scene.materials().add(&Material::new()
            .with_class(MaterialClass::Mirror)
            .with_albedo(0.64, 0.77, 0.22)
            .with_refractive_index_eta(1.4));

        let light_material = scene.materials().add(&Material::new()
            .with_emission(2.0, 2.0, 2.0));

        let black_material = scene.materials().add(&Material::new()
            .with_albedo(0.2, 0.2, 0.2)
            .with_specular(0.2, 0.2, 0.2)
            .with_specular_strength(0.05)
            .with_roughness(0.95));

        let coral_material = scene.materials().add(&Material::new()
            .with_albedo(1.0, 0.5, 0.3)
            .with_specular(0.2, 0.2, 0.2)
            .with_specular_strength(0.01)
            .with_roughness(0.0));

        let red_material = scene.materials().add(&Material::new()
            .with_albedo(0.75, 0.1, 0.1)
            .with_specular(0.75, 0.1, 0.1)
            .with_specular_strength(0.05)
            .with_roughness(0.95));

        let bright_red_material = scene.materials().add(&Material::new()
            .with_albedo(1.0, 0.0, 0.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(0.05)
            .with_roughness(0.95));

        let silver_material = scene.materials().add(&Material::new()
            .with_class(MaterialClass::Mirror)
            .with_albedo(0.75, 0.75, 0.75)
            .with_specular(0.75, 0.75, 0.75)
            .with_specular_strength(0.55)
            .with_roughness(0.0));

        let green_material = scene.materials().add(&Material::new()
            .with_albedo(0.05, 0.55, 0.05)
            .with_specular(0.05, 0.55, 0.05)
            .with_specular_strength(0.05)
            .with_roughness(0.95));

        let selected_object_material = scene.materials().add(&Material::new()
            .with_albedo(0.05, 0.05, 2.05)
            .with_specular(0.05, 0.05, 0.55)
            .with_specular_strength(0.15)
            .with_roughness(0.45));

        scene.add_sdf_box(
            &(Affine::from_translation(Vector::new(0.7, 0.2, -0.7))*Affine::from_angle_z(Deg(-30.0))),
            Vector::new(0.24, 0.1, 0.02),
            0.03,
            silver_material);

        scene.add_sdf_box(
            &(Affine::from_translation(Vector::new(1.5, -0.4, -0.9))*Affine::from_angle_z(Deg(30.0))),
            Vector::new(0.24, 0.1, 0.02),
            0.03,
            bright_red_material);

        scene.add_sphere(Point::new(1.5, 0.6, -1.0), 0.25, gold_metal);
        scene.add_sphere(Point::new(0.5, 0.0, -1.0), 0.25, blue_glass);
        scene.add_sphere(Point::new(1.5, 0.0, -1.0), 0.25, green_mirror);
        scene.add_sphere(Point::new(0.0, 0.0, -1.0), 0.25, coral_material);

        scene.add_parallelogram(Point::new(-1.0, 1.0, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0), light_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), black_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), red_material);
        scene.add_parallelogram(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), green_material);

        #[must_use]
        fn get_resource_path(file_name: impl AsRef<Path>) -> PathBuf {
            let exe_path = env::current_exe().unwrap();
            let exe_directory = exe_path.parent().unwrap();
            exe_directory.join(file_name)
        }
        
        let mut meshes = MeshWarehouse::new();
        let cube_mesh_file = get_resource_path(Path::new("assets").join("cube.obj"));
        let cube_mesh_or_error = meshes.load(cube_mesh_file);

        match cube_mesh_or_error {
            Ok(cube_mesh) => {
                let large_box_material = scene.materials().add(&Material::new()
                    .with_albedo(0.95, 0.95, 0.95)
                    .with_refractive_index_eta(2.5));
                let large_box_location =
                    Transformation::new(
                        Affine::from_translation(Vector::new(0.15, 0.6, -1.0)) *
                            Affine::from_nonuniform_scale(3.65, 0.8, 0.25));
                scene.add_mesh(&meshes, cube_mesh, &large_box_location, large_box_material);

                {
                    let box_location =Transformation::new(
                        Affine::from_translation(Vector::new(-0.4, 0.1, -1.0)) * Affine::from_scale(0.4));
                    scene.add_mesh(&meshes, cube_mesh, &box_location, gold_metal);
                }

                {
                    let box_location = Transformation::new(
                        Affine::from_translation(Vector::new(0.9, -0.4, -1.0)) * Affine::from_scale(0.4));
                    scene.add_mesh(&meshes, cube_mesh, &box_location, purple_glass);
                }

                {
                    let box_location = Transformation::new(
                        Affine::from_translation(Vector::new(0.4, 0.1, 0.2)) * Affine::from_nonuniform_scale(0.9, 0.9, 0.1));
                    scene.add_mesh(&meshes, cube_mesh, &box_location, red_glass);
                }
            },
            Err(mesh_loading_error) => {
                error!("failed to load cube mesh: {}", mesh_loading_error);
            },
        }

        let engine = pollster::block_on(Engine::new(window.clone(), scene, camera))?;
        
        Ok(Self { 
            engine, 
            left_mouse_down: false, 
            last_cursor_position: None, 
            selected_object: None,
            selected_object_material,
        })
    }
}

