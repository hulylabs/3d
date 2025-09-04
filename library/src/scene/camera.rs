use crate::geometry::alias::{Point, Vector};
use crate::geometry::transform::Affine;
use crate::serialization::serialize_matrix::serialize_matrix_4x4;
use cgmath::{Deg, EuclideanSpace, InnerSpace, SquareMatrix, Transform, Vector3, Zero};
use std::ops::Mul;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

#[must_use]
fn projection_into_point(projection_target: Point) -> Affine {
    Affine::new(
        0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
        projection_target.x, projection_target.y, projection_target.z, 1.0,
    )
}

#[must_use]
fn projection_into_plane(plane_point: Point, plane_normal: Vector) -> Affine {
    let plane_normal = plane_normal.normalize();
    let outer_product_normal = Affine::new(
        plane_normal.x * plane_normal.x, plane_normal.x * plane_normal.y, plane_normal.x * plane_normal.z, 0.0,
        plane_normal.y * plane_normal.x, plane_normal.y * plane_normal.y, plane_normal.y * plane_normal.z, 0.0,
        plane_normal.z * plane_normal.x, plane_normal.z * plane_normal.y, plane_normal.z * plane_normal.z, 0.0,
        0.0, 0.0, 0.0, 0.0,
    );
    let local_projection = Affine::identity() - outer_product_normal;

    let translation_to_origin = Affine::from_translation(-plane_point.to_vec());
    let translation_back = Affine::from_translation(plane_point.to_vec());

    translation_back * local_projection * translation_to_origin
}

pub trait CameraKind {
    #[must_use]
    fn ray_origin(&self, eye: Point, look_at: Point) -> Affine;
    #[must_use]
    fn box_clone(&self) -> Box<dyn CameraKind>;
}

pub struct PerspectiveCamera;
impl CameraKind for PerspectiveCamera {
    fn ray_origin(&self, eye: Point, _look_at: Point) -> Affine {
        projection_into_point(eye)
    }
    fn box_clone(&self) -> Box<dyn CameraKind> {
        Box::new(Self{})
    }
}

pub struct OrthographicCamera;
impl CameraKind for OrthographicCamera {
    fn ray_origin(&self, eye: Point, look_at: Point) -> Affine {
        projection_into_plane(eye, look_at - eye)
    }
    fn box_clone(&self) -> Box<dyn CameraKind> {
        Box::new(Self{})
    }
}

pub struct Camera {
    world_to_camera_space: Affine,
    view_ray_origin: Affine,

    kind: Box<dyn CameraKind>,

    horizontal_rotation: Deg<f64>,
    vertical_rotation: Deg<f64>,
    eye_rod_length: f64,
    look_at: Point,
    eye_offset: Vector3<f64>,

    updated: bool,
    zoom_speed: f64,
    linear_speed: f64,
    rotation_speed: Deg<f64>,
}

const MIN_ROD_LENGTH: f64 = 0.01;

impl Camera {
    #[must_use]
    fn new(eye_rod_length: f64, kind: Box<dyn CameraKind>, look_at: Point) -> Self {
        assert!(eye_rod_length > 0.0);
        let mut result = Camera {
            world_to_camera_space: Affine::identity(),
            view_ray_origin: Affine::identity(),
            kind,
            horizontal_rotation: Deg::zero(),
            vertical_rotation: Deg::zero(),
            eye_rod_length,
            look_at,
            eye_offset: Vector3::zero(),
            updated: false,
            zoom_speed: 1.0,
            linear_speed: 1.0,
            rotation_speed: Deg(1.0),
        };
        result.build();
        result
    }

    pub fn set_from(&mut self, other: &Camera) {
        self.world_to_camera_space = other.world_to_camera_space;
        self.view_ray_origin = other.view_ray_origin;

        self.kind = other.kind.box_clone();

        self.horizontal_rotation = other.horizontal_rotation;
        self.vertical_rotation = other.vertical_rotation;
        self.eye_rod_length = other.eye_rod_length;
        self.look_at = other.look_at;
        self.eye_offset = other.eye_offset;

        self.updated = other.updated;
        self.zoom_speed = other.zoom_speed;
        self.linear_speed = other.linear_speed;
        self.rotation_speed = other.rotation_speed;
        
        self.updated = true;
    }

    #[must_use]
    pub fn new_perspective_camera(eye_rod_length: f64, look_at: Point) -> Self {
        assert!(eye_rod_length > 0.0);
        Self::new(eye_rod_length, Box::new(PerspectiveCamera{}), look_at)
    }

    #[must_use]
    pub fn new_orthographic_camera(eye_rod_length: f64, look_at: Point) -> Self {
        assert!(eye_rod_length > 0.0);
        Self::new(eye_rod_length, Box::new(OrthographicCamera{}), look_at)
    }

    #[must_use]
    pub(crate) fn check_and_clear_updated_status(&mut self) -> bool {
        let result = self.updated;
        self.updated = false;
        result
    }

    fn build(&mut self) {
        let horizontal_rotation = Affine::from_angle_y(self.horizontal_rotation);
        let vertical_rotation = Affine::from_angle_x(self.vertical_rotation);

        let rotation = horizontal_rotation * vertical_rotation;

        let eye = Point::new(0.0, 0.0, self.eye_rod_length);
        let eye = rotation.transform_point(eye);
        let eye = eye + Vector::new(self.eye_offset.x, self.eye_offset.y, self.eye_offset.z);

        let up = Vector::new(0.0, 1.0, 0.0);
        let up = rotation.transform_vector(up);

        let look_at = self.look_at;

        self.world_to_camera_space = Affine::look_at_rh(eye, look_at, up);
        self.view_ray_origin = self.kind.ray_origin(eye, look_at);
    }

    fn mark_updated_and_build(&mut self) {
        self.updated = true;
        self.build();
    }

    pub fn set_zoom_speed(&mut self, per_unit: f64) {
        self.zoom_speed = per_unit;
    }

    pub fn set_linear_speed(&mut self, per_unit: f64) {
        self.linear_speed = per_unit;
    }

    pub fn set_rotation_speed(&mut self, degrees_per_unit: Deg<f64>) {
        self.rotation_speed = degrees_per_unit;
    }

    pub fn move_horizontally(&mut self, delta: f64) {
        let actual_delta = delta * self.linear_speed;
        self.eye_offset.x += actual_delta;
        self.look_at.x += actual_delta;
        self.mark_updated_and_build();
    }

    pub fn move_vertically(&mut self, delta: f64) {
        let actual_delta = delta * self.linear_speed;
        self.eye_offset.y += actual_delta;
        self.look_at.y += actual_delta;
        self.mark_updated_and_build();
    }

    pub fn move_depth_wise(&mut self, delta: f64) {
        let actual_delta = delta * self.linear_speed;
        self.eye_offset.z += actual_delta;
        self.look_at.z += actual_delta;
        self.mark_updated_and_build();
    }

    pub fn zoom(&mut self, delta: f64) {
        let actual_delta = delta * self.zoom_speed;
        if self.eye_rod_length + actual_delta < MIN_ROD_LENGTH {
            return;
        }
        self.eye_rod_length += actual_delta;
        self.mark_updated_and_build();
    }

    pub fn rotate_horizontal(&mut self, units: f64) {
        self.horizontal_rotation += self.rotation_speed.mul(units);
        self.mark_updated_and_build();
    }

    pub fn rotate_vertical(&mut self, units: f64) {
        self.vertical_rotation += self.rotation_speed.mul(units);
        self.mark_updated_and_build();
    }

    pub fn set_kind(&mut self, kind: Box<dyn CameraKind>) {
        self.kind = kind;
        self.mark_updated_and_build();
    }

    pub(crate) const SERIALIZED_QUARTET_COUNT: usize = 8;

    pub(crate) fn serialize_into(&self, container: &mut GpuReadySerializationBuffer) {
        assert!(container.free_quartets_of_current_object() >= Camera::SERIALIZED_QUARTET_COUNT, "buffer size is too small");

        let camera_space_to_world = self.world_to_camera_space.invert().unwrap();
        let view_ray_origin = self.view_ray_origin;

        serialize_matrix_4x4(container, &camera_space_to_world);
        serialize_matrix_4x4(container, &view_ray_origin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::assert_abs_diff_eq;
    use std::f32::consts::FRAC_1_SQRT_2;
    use bytemuck::cast_slice;
    use crate::serialization::gpu_ready_serialization_buffer::ELEMENTS_IN_QUARTET;

    fn assert_camera_serialized_data(system_under_test: &Camera, expected_data: [f32; Camera::SERIALIZED_QUARTET_COUNT * ELEMENTS_IN_QUARTET]) {
        let mut container = GpuReadySerializationBuffer::new(1, Camera::SERIALIZED_QUARTET_COUNT);
        system_under_test.serialize_into(&mut container);
        assert_abs_diff_eq!(cast_slice(container.backend()), &expected_data[..]);
    }

    #[test]
    fn test_new_perspective_camera() {
        let z_axis_offset = 0.7;
        let mut system_under_test = Camera::new_perspective_camera(z_axis_offset, Point::origin());

        let expected_serialized_camera = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, z_axis_offset as f32, 1.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, z_axis_offset as f32, 1.0,
        ];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
        assert_eq!(false, system_under_test.check_and_clear_updated_status());
    }

    #[test]
    fn test_check_and_clear_updated_status() {
        let mut system_under_test = Camera::new(1.0, Box::new(PerspectiveCamera), Point::origin());
        system_under_test.rotate_horizontal(90.0);

        assert!(system_under_test.check_and_clear_updated_status());
        assert_eq!(false, system_under_test.check_and_clear_updated_status());
    }

    #[test]
    fn test_rotate_horizontal() {
        let mut system_under_test = Camera::new_perspective_camera(1.0, Point::origin());
        system_under_test.rotate_horizontal(90.0);

        let expected_serialized_camera = [
            0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
            0.0, 0.0,  0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0,
        ];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
        assert!(system_under_test.check_and_clear_updated_status());
    }

    #[test]
    fn test_move_horizontally() {
        let z_axis_offset = 1.7;
        let mut system_under_test = Camera::new_perspective_camera(z_axis_offset, Point::origin());

        system_under_test.move_horizontally(13.0);
        let expected_serialized_camera = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 13.0, 0.0, z_axis_offset as f32, 1.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 13.0, 0.0, z_axis_offset as f32, 1.0,];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
        assert!(system_under_test.check_and_clear_updated_status());

        system_under_test.move_horizontally(-26.0);
        let expected_serialized_camera = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -13.0, 0.0, z_axis_offset as f32, 1.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -13.0, 0.0, z_axis_offset as f32, 1.0];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
        assert!(system_under_test.check_and_clear_updated_status());
    }

    #[test]
    fn test_set_zoom_speed() {
        let initial_z_offset = 1.0;
        let mut system_under_test = Camera::new_perspective_camera(initial_z_offset, Point::origin());
        let additional_offset = 3.0;

        system_under_test.set_zoom_speed(3.0);
        system_under_test.zoom(1.0);

        let expected_offset = (initial_z_offset + additional_offset) as f32;
        let expected_serialized_camera = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, expected_offset, 1.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, expected_offset, 1.0,
        ];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
    }

    #[test]
    fn test_set_linear_speed() {
        let initial_z_offset = 1.0;
        let mut system_under_test = Camera::new_perspective_camera(1.0, Point::origin());
        let expected_x_offset = -3.0;

        system_under_test.set_linear_speed(expected_x_offset);
        system_under_test.move_horizontally(1.0);

        let expected_serialized_camera = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, expected_x_offset as f32, 0.0, initial_z_offset, 1.0,
            0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, expected_x_offset as f32, 0.0, initial_z_offset, 1.0,
        ];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
    }

    #[test]
    fn test_set_rotation_speed() {
        let mut system_under_test = Camera::new_perspective_camera(1.0, Point::origin());

        system_under_test.set_rotation_speed(Deg(-45.0));
        system_under_test.rotate_vertical(1.0);

        let expected_serialized_camera: [f32; Camera::SERIALIZED_QUARTET_COUNT * ELEMENTS_IN_QUARTET] =
        [
            1.0, 0.0, 0.0, 0.0,
            0.0, FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0.0,
            0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.0,
            0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2, 1.0,

            0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
            0.0, 0.0, 0.0, 0.0,
            0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2, 1.0
        ];
        assert_camera_serialized_data(&system_under_test, expected_serialized_camera);
    }

    #[test]
    fn test_projection_into_point() {
        let projection_target = Point::new(1.0, 2.0, 3.0);
        let projection = projection_into_point(projection_target);
        assert_eq!(projection_target, projection.transform_point(Point::new(0.0, 0.0, 0.0)));
        assert_eq!(projection_target, projection.transform_point(Point::new(1.0, 0.0, 0.0)));
        assert_eq!(projection_target, projection.transform_point(Point::new(0.0, 1.0, 0.0)));
        assert_eq!(projection_target, projection.transform_point(Point::new(0.0, 0.0, 1.0)));
        assert_eq!(projection_target, projection.transform_point(Point::new(1.0, 1.0, 1.0)));
        assert_eq!(projection_target, projection.transform_point(projection_target));
    }

    #[test]
    fn test_projection_into_plane() {
        let projection = projection_into_plane(Point::new(1.0, 1.0, 0.0), Vector::new(2.0, 0.0, 0.0));

        assert_eq!(projection.transform_point(Point::new(0.0, 0.0, 0.0)), Point::new(1.0, 0.0, 0.0));
        assert_eq!(projection.transform_point(Point::new(1.0, 1.0, 1.0)), Point::new(1.0, 1.0, 1.0));
        assert_eq!(projection.transform_point(Point::new(2.0, 2.0, 2.0)), Point::new(1.0, 2.0, 2.0));
    }
}
