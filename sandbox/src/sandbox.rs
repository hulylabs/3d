use crate::world::{Materials, SdfClasses, World};
use cgmath::Deg;
use library::geometry::alias::Point;
use library::objects::material_index::MaterialIndex;
use library::scene::camera::{Camera, OrthographicCamera, PerspectiveCamera};
use library::scene::container::Container;
use library::sdf::code_generator::SdfRegistrator;
use library::utils::object_uid::ObjectUid;
use library::Engine;
use std::sync::Arc;
use log::info;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;
use library::utils::min_max_time_measurer::MinMaxTimeMeasurer;
use crate::beautiful_world::{BeautifulMaterials, BeautifulSdfClasses, BeautifulWorld};

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
    tech_world: World,
    beautiful_world: BeautifulWorld,
    
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
        } else if MouseButton::Middle == button && state == ElementState::Pressed {
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
                } else if "m" == letter_key {
                    self.engine.use_monte_carlo_render();
                } else if "n" == letter_key {
                    self.engine.use_deterministic_render();
                } else if "+" == letter_key {
                    self.tech_world.move_light_z(1.0, self.engine.scene());
                } else if "-" == letter_key {
                    self.tech_world.move_light_z(-1.0, self.engine.scene());
                } else if "*" == letter_key {
                    self.tech_world.move_light_x(1.0, self.engine.scene());
                } else if "/" == letter_key {
                    self.tech_world.move_light_x(-1.0, self.engine.scene());
                } else if "1" == letter_key {
                    self.tech_world.switch_to_ui_box_scene(self.engine.scene());
                    self.selected_object = None;
                } else if "2" == letter_key {
                    self.tech_world.switch_to_sdf_exhibition_scene(self.engine.scene());
                    self.selected_object = None;
                } else if "3" == letter_key {
                    self.tech_world.switch_to_constructive_solid_geometry_sample_scene(self.engine.scene());
                    self.selected_object = None;
                } else if "4" == letter_key {
                    self.tech_world.switch_to_smooth_operators_scene(self.engine.scene());
                    self.selected_object = None;
                } else if "5" == letter_key {
                    self.beautiful_world.create_crystal_palace_scene(self.engine.scene());
                    self.selected_object = None;
                } else if "6" == letter_key {
                    self.beautiful_world.create_underwater_treasure_scene(self.engine.scene());
                    self.selected_object = None;
                } else if "7" == letter_key {
                    self.beautiful_world.create_zen_garden_scene(self.engine.scene());
                    self.selected_object = None;
                }
            }
            _ => (),
        }
    }
    
    pub(super) fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let mut timer = MinMaxTimeMeasurer::new();
        
        timer.start();
        
        let camera = make_default_camera();
        
        let mut sdf_registrator = SdfRegistrator::default();
        let tech_sdf_classes = SdfClasses::new(&mut sdf_registrator);
        let beautiful_sdf_classes = BeautifulSdfClasses::new(&mut sdf_registrator);
        
        let mut scene = Container::new(sdf_registrator);
        let tech_materials = Materials::new(&mut scene);
        let beautiful_materials = BeautifulMaterials::new(&mut scene);
        
        let mut tech_world = World::new(tech_sdf_classes, tech_materials);
        tech_world.switch_to_ui_box_scene(&mut scene);
        let selected_object_material = tech_world.selected_object_material();
        
        let beautiful_world = BeautifulWorld::new(beautiful_sdf_classes, beautiful_materials);
        
        let engine = pollster::block_on(Engine::new(window.clone(), scene, camera))?;
        
        timer.stop();
        info!("sandbox initialized in {} seconds", timer.max_time().as_secs_f64());
        
        Ok(Self { 
            engine,
            tech_world,
            beautiful_world,
            left_mouse_down: false, 
            last_cursor_position: None, 
            selected_object: None,
            selected_object_material,
        })
    }
}

