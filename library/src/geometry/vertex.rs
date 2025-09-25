use crate::geometry::alias::{Point, Vector};
use crate::geometry::epsilon::DEFAULT_EPSILON_F64;
use cgmath::AbsDiffEq;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Vertex {
    position: Point,
    normal: Vector,
}

impl AbsDiffEq for Vertex {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        DEFAULT_EPSILON_F64
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Point::abs_diff_eq(&self.position, &other.position, epsilon) && Vector::abs_diff_eq(&self.normal, &other.normal, epsilon)
    }
}

impl Vertex {
    #[must_use]
    pub(crate) fn new(position: Point, normal: Vector) -> Vertex {
        Vertex { position, normal }
    }

    #[must_use]
    pub(crate) fn position(&self) -> Point {
        self.position
    }

    #[must_use]
    pub(crate) fn normal(&self) -> Vector {
        self.normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::{assert_abs_diff_eq, EuclideanSpace};

    #[test]
    fn test_vertex_new() {
        let position = Point::new(1.0, 2.0, 3.0);
        let normal = Vector::new(4.0, 5.0, 6.0);

        let system_under_test = Vertex::new(position, normal);

        assert_eq!(system_under_test.position(), position);
        assert_eq!(system_under_test.normal(), normal);
    }

    #[test]
    fn test_epsilon_equality() {
        let position = Point::new(1.0, 2.0, 3.0);
        let normal = Vector::new(4.0, 5.0, 6.0);

        let system_under_test = Vertex::new(position, normal);

        let epsilon = 0.2;
        let per_component_epsilon = Vector::new(epsilon / 2.0, epsilon / 2.0, epsilon / 2.0);
        let expected_position = position + per_component_epsilon;
        let expected_normal = normal + per_component_epsilon;
        assert_abs_diff_eq!(system_under_test.position(), expected_position, epsilon = epsilon);
        assert_abs_diff_eq!(system_under_test.normal(), expected_normal, epsilon = epsilon);
    }

    #[test]
    fn test_vertex_position() {
        let expected_position = Point::new(1.0, 2.0, 3.0);

        let system_under_test = Vertex::new(expected_position, Vector::unit_x());

        assert_eq!(system_under_test.position(), expected_position);
    }

    #[test]
    fn test_vertex_normal() {
        let expected_normal = Vector::new(1.0, 2.0, 3.0);

        let system_under_test = Vertex::new(Point::origin(), expected_normal);

        assert_eq!(system_under_test.normal(), expected_normal);
    }

    #[test]
    fn test_abs_diff_eq_with_tolerance() {
        let left = Vertex::new(
            Point::new(1.0, 2.0, 3.0),
            Vector::new(0.0, 1.0, 0.0));
        let right = Vertex::new(
            Point::new(1.0 + Vertex::default_epsilon() / 2.0, 2.0, 3.0),
            Vector::new(0.0, 1.0 + Vertex::default_epsilon() / 2.0, 0.0));

        assert!(left.abs_diff_eq(&right, Vertex::default_epsilon()));
    }

    #[test]
    fn test_abs_diff_eq_outside_tolerance() {
        let left = Vertex::new(
            Point::new(1.0, 2.0, 3.0),
            Vector::new(0.0, 1.0, 0.0));
        let right = Vertex::new(
            Point::new(1.0 + Vertex::default_epsilon() * 2.0, 2.0, 3.0),
            Vector::new(0.0, 1.0 + Vertex::default_epsilon() * 2.0, 0.0));

        assert!(!left.abs_diff_eq(&right, Vertex::default_epsilon()));
    }

    #[test]
    fn test_abs_diff_eq_partial_match() {
        let left = Vertex::new(
            Point::new(1.0, 2.0, 3.0),
            Vector::new(0.0, 1.0, 0.0));

        let right = Vertex::new(
            Point::new(1.0, 2.0 + Vertex::default_epsilon() * 2.0, 3.0),
            Vector::new(0.0, 1.0, 0.0));

        assert!(!left.abs_diff_eq(&right, Vertex::default_epsilon()));
    }
}
