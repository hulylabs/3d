use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::Affine;
use crate::serialization::helpers::{GpuFloatBufferFiller, floats_count};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use cgmath::{Deg, EuclideanSpace, Quaternion, Rotation, Rotation3, SquareMatrix, Vector2, Zero};

// TODO: rewrite this

#[derive(Copy, Clone)]
pub struct Camera {
    view_matrix: Affine,
    eye: Point,
    look_at: Point,
    up: Vector,
    zoom_direction: Vector,
    rotate_angle: Deg<f64>,
    zoom_speed: f64,
    move_speed: f64,
    keypress_move_speed: f64,
    moving: bool,
    key_press: bool,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            // TODO: add meaningful default values
            view_matrix: Affine::identity(),
            eye: Point::origin(),
            look_at: Point::origin(),
            up: Vector::zero(),
            zoom_direction: Vector::zero(),
            rotate_angle: Deg::zero(),
            zoom_speed: 0.1,
            move_speed: 0.01,
            keypress_move_speed: 0.1,
            moving: false,
            key_press: false,
        }
    }

    pub fn set(&mut self, eye: Option<Point>, look_at: Option<Point>, up: Option<Vector>) {
        if let Some(eye) = eye {
            self.eye = eye;
        }
        if let Some(look_at) = look_at {
            self.look_at = look_at;
        }
        if let Some(up) = up {
            self.up = up;
        }

        self.zoom_direction = self.eye - self.look_at;
        self.view_matrix = Affine::look_at_rh(self.eye, self.look_at, self.up); // TODO: sync with the JS
    }

    pub fn zoom(&mut self, delta: f64) {
        let sign = delta.signum();
        self.eye += self.zoom_direction * self.zoom_speed * sign;
        self.set(None, None, None);
        self.key_press = true;
    }

    // TODO: totally rewrite (duplicates, evaluations)

    pub fn move_camera(&mut self, old_coord: Vector2<f64>, new_coord: Vector2<f64>) {
        let d_x = (new_coord[0] - old_coord[0]) * std::f64::consts::PI / 180.0 * self.move_speed;

        self.rotate_angle = Deg(d_x);

        let rotation = Quaternion::from_angle_y(self.rotate_angle);
        self.eye = rotation.rotate_point(self.eye);

        self.set(None, None, None);

        self.key_press = true;
    }

    pub fn move_left(&mut self) {
        self.eye.x += self.keypress_move_speed;
        self.look_at.x += self.keypress_move_speed;
        self.set(None, None, None);
        self.key_press = true;
    }

    pub fn move_right(&mut self) {
        self.eye.x -= self.keypress_move_speed;
        self.look_at.x -= self.keypress_move_speed;
        self.set(None, None, None);
        self.key_press = true;
    }

    pub fn move_up(&mut self) {
        self.eye.y -= self.keypress_move_speed;
        self.look_at.y -= self.keypress_move_speed;
        self.set(None, None, None);
        self.key_press = true;
    }

    pub fn move_down(&mut self) {
        self.eye.y += self.keypress_move_speed;
        self.look_at.y += self.keypress_move_speed;
        self.set(None, None, None);
        self.key_press = true;
    }

    pub fn moving(&self) -> bool {
        self.moving
    }

    pub fn key_press(&self) -> bool {
        self.key_press
    }

    pub fn set_key_press(&mut self, key_press: bool) {
        self.key_press = key_press;
    }

    pub(crate)  const SERIALIZED_QUARTET_COUNT: usize = 4;
}

impl SerializableForGpu for Camera {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Camera::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Camera::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        // TODO: this is to comply with the shader code - rewrite, so we can use ortho matrix in shader

        let mut index = 0;
        container.write_and_move_next(1.0, &mut index);
        container.write_and_move_next(0.0, &mut index);
        container.write_and_move_next(0.0, &mut index);
        container.write_and_move_next(0.0, &mut index);

        container.write_and_move_next(0.0, &mut index);
        container.write_and_move_next(1.0, &mut index);
        container.write_and_move_next(0.0, &mut index);
        container.write_and_move_next(0.0, &mut index);

        container.write_and_move_next(0.0, &mut index);
        container.write_and_move_next(0.0, &mut index);
        container.write_and_move_next(1.0, &mut index);
        container.write_and_move_next(0.0, &mut index);

        container.write_and_move_next(self.eye.x, &mut index);
        container.write_and_move_next(self.eye.y, &mut index);
        container.write_and_move_next(self.eye.z, &mut index);
        container.write_and_move_next(1.0, &mut index);

        assert_eq!(index, Camera::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_into() {
        let mut system_under_test = Camera::new();
        system_under_test.set(Some(Point::new(0.5, 0.0, 0.8)), Some(Point::new(0.5, 0.0, 0.0)), Some(Vector::new(0.0, 1.0, 0.0)));
        let container_initial_filler = -1.0;

        let mut container = vec![container_initial_filler; Camera::SERIALIZED_SIZE_FLOATS + 1];
        system_under_test.serialize_into(&mut container);

        let expected_data = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.8, 1.0, container_initial_filler];

        assert_eq!(container, &expected_data);
    }
}
