use crate::geometry::alias::{Point, Vector};
use crate::serialization::filler::{floats_count, GpuFloatBufferFiller};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use cgmath::{InnerSpace, Matrix, Matrix4, SquareMatrix, Transform};

pub type Affine = Matrix4<f64>;

pub struct Transformation {
    forward: Affine,
    inverse: Affine,
}

impl Transformation {
    #[must_use]
    pub fn identity() -> Self {
        Transformation {
            forward: Affine::identity(),
            inverse: Affine::identity(),
        }
    }

    #[must_use]
    pub fn new(source: Affine) -> Self {
        Transformation {
            forward: source,
            inverse: source.invert().unwrap_or(Affine::identity()),
        }
    }

    #[must_use]
    pub(crate) fn of_point(&self, target: Point) -> Point {
        self.forward.transform_point(target)
    }

    #[must_use]
    pub(crate) fn of_surface_vector(&self, target: Vector) -> Vector {
        self.inverse.transpose().transform_vector(target).normalize()
    }

    const SERIALIZED_QUARTET_COUNT: usize = 8;
}

impl SerializableForGpu for Transformation {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(Transformation::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= Transformation::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;

        for column in 0..4 {
            for row in 0..4 {
                container.write_and_move_next(self.forward[column][row], &mut index);
            }
        }

        for column in 0..4 {
            for row in 0..4 {
                container.write_and_move_next(self.inverse[column][row], &mut index);
            }
        }

        assert_eq!(index, Transformation::SERIALIZED_SIZE_FLOATS);
    }
}

pub(crate) trait TransformableCoordinate {
    fn new(x: f64, y: f64, z: f64) -> Self;
    fn transform(self, transformation: &Transformation) -> Self;
}

impl TransformableCoordinate for Point {
    #[must_use]
    fn new(x: f64, y: f64, z: f64) -> Self {
        Point::new(x, y, z)
    }

    #[must_use]
    fn transform(self, transformation: &Transformation) -> Self {
        transformation.of_point(self)
    }
}

impl TransformableCoordinate for Vector {
    #[must_use]
    fn new(x: f64, y: f64, z: f64) -> Self {
        Vector::new(x, y, z)
    }

    #[must_use]
    fn transform(self, transformation: &Transformation) -> Self {
        transformation.of_surface_vector(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{assert_abs_diff_eq, InnerSpace, Rad};
    use std::f64::consts::PI;
    use crate::geometry::epsilon::DEFAULT_EPSILON;

    #[test]
    fn test_of_point() {
        let affine =
              Affine::from_translation(Vector::new(1.0, 2.0, 3.0))
            * Affine::from_nonuniform_scale(3.0, 2.0, 1.0);
        let victim_point = Point::new(1.0, 1.0, 1.0);
        let expected_point = affine.transform_point(victim_point);

        let system_under_test = Transformation::new(affine);

        let actual_point = system_under_test.of_point(victim_point);

        assert_eq!(actual_point, expected_point);
    }

    #[test]
    fn test_of_surface_vector() {
        let affine =
              Affine::from_translation(Vector::new(3.0, 2.0, 1.0))
            * Affine::from_scale(3.0);
        let victim_vector = Vector::new(1.0, 1.0, 1.0).normalize();
        let expected_vector = victim_vector.clone();

        let system_under_test = Transformation::new(affine);

        let actual_vector = system_under_test.of_surface_vector(victim_vector);

        assert_abs_diff_eq!(actual_vector, expected_vector, epsilon = DEFAULT_EPSILON);
    }

    #[test]
    fn test_serializable_for_gpu() {
        let source = Affine::from_nonuniform_scale(1.0, 2.0, 3.0);
        let system_under_test = Transformation::new(source);
        let container_initial_filler = -7.0;
        let serialized_size = Transformation::SERIALIZED_SIZE_FLOATS + 1;
        let mut container = vec![container_initial_filler; serialized_size];

        system_under_test.serialize_into(&mut container);

        let mut expected_container = vec![0.0; serialized_size];
        expected_container[ 0] = 1.0;
        expected_container[ 5] = 2.0;
        expected_container[10] = 3.0;
        expected_container[15] = 1.0;
        expected_container[16] = 1.0;
        expected_container[21] = 1.0 / 2.0;
        expected_container[26] = 1.0 / 3.0;
        expected_container[31] = 1.0;
        expected_container[32] = container_initial_filler;

        assert_eq!(container, expected_container);
    }

    #[test]
    fn test_transformable_coordinate_point() {
        let transformation =
            Affine::from_nonuniform_scale(1.0, 2.0, 3.0) *
            Affine::from_angle_z(Rad(PI / 2.0)) *
            Affine::from_translation(Vector::new(1.0, 0.0, 0.0))
            ;
        let victim_point = Point::new(0.0, 0.0, 0.0);
        let expected_point = Point::new(0.0, 2.0, 0.0);
        let system_under_test = Transformation::new(transformation);

        let actual_point = victim_point.transform(&system_under_test);

        assert_abs_diff_eq!(actual_point, expected_point, epsilon = DEFAULT_EPSILON);
    }

    #[test]
    fn test_transformable_coordinate_vector() {
        let transformation =
            Affine::from_translation(Vector::new(1.0, 0.0, 0.0)) *
            Affine::from_angle_z(Rad(PI / 2.0)) *
            Affine::from_nonuniform_scale(1.0, 2.0, 3.0);
        let victim_vector = Vector::new(1.0, 0.0, 0.0);
        let expected_vector = Vector::new(0.0, 1.0, 0.0);
        let system_under_test = Transformation::new(transformation);

        let actual_vector = victim_vector.transform(&system_under_test);

        assert_abs_diff_eq!(actual_vector, expected_vector, epsilon = DEFAULT_EPSILON);
    }
}