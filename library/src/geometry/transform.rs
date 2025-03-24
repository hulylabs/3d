use cgmath::{Matrix, Matrix4, Transform};
use cgmath::SquareMatrix;
use crate::geometry::alias::{Point, Vector};

pub(crate) type Affine = Matrix4<f32>;

pub(crate) struct Transformation {
    forward: Affine,
    inverse: Affine,
}

impl Transformation {
    #[must_use]
    pub(crate) fn of_point(&self, target: &Point) -> Point {
        self.forward.transform_point(*target)
    }

    #[must_use]
    pub(crate) fn of_surface_vector(&self, target: &Vector) -> Vector {
        self.inverse.transpose().transform_vector(*target)
    }
}

impl Transformation {
    #[must_use]
    pub(crate) fn new(source: Affine) -> Self {
        Transformation {
            forward: source,
            inverse: source.invert().unwrap_or(Affine::identity()),
        }
    }
}