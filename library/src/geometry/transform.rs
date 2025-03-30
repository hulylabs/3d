use crate::geometry::alias::{Point, Vector};
use crate::geometry::axis::Axis;
use crate::serialization::helpers::{GpuFloatBufferFiller, floats_count};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use cgmath::SquareMatrix;
use cgmath::{Matrix, Matrix4, Transform};
use strum::EnumCount;

pub type Affine = Matrix4<f32>;

pub(crate) struct Transformation {
    forward: Affine,
    inverse: Affine,
}

impl Transformation {
    #[must_use]
    pub(crate) fn identity() -> Self {
        Transformation {
            forward: Affine::identity(),
            inverse: Affine::identity(),
        }
    }

    #[must_use]
    pub(crate) fn new(source: Affine) -> Self {
        Transformation {
            forward: source,
            inverse: source.invert().unwrap_or(Affine::identity()),
        }
    }

    #[must_use]
    pub(crate) fn of_point(&self, target: &Point) -> Point {
        self.forward.transform_point(*target)
    }

    #[must_use]
    pub(crate) fn of_surface_vector(&self, target: &Vector) -> Vector {
        self.inverse.transpose().transform_vector(*target)
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
    fn new(x: f32, y: f32, z: f32) -> Self;
    fn transform(self, transformation: &Affine) -> Self;
    fn to_array(self) -> [f32; Axis::COUNT];
}

impl TransformableCoordinate for Point {
    #[must_use]
    fn new(x: f32, y: f32, z: f32) -> Self {
        Point::new(x, y, z)
    }

    #[must_use]
    fn transform(self, transformation: &Affine) -> Self {
        transformation.transform_point(self)
    }

    #[must_use]
    fn to_array(self) -> [f32; Axis::COUNT] {
        [self.x, self.y, self.z]
    }
}

impl TransformableCoordinate for Vector {
    #[must_use]
    fn new(x: f32, y: f32, z: f32) -> Self {
        Vector::new(x, y, z)
    }

    #[must_use]
    fn transform(self, transformation: &Affine) -> Self {
        transformation.transform_vector(self)
    }

    #[must_use]
    fn to_array(self) -> [f32; Axis::COUNT] {
        [self.x, self.y, self.z]
    }
}
