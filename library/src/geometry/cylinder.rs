use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::geometry::axis::Axis;
use cgmath::EuclideanSpace;
use more_asserts::assert_gt;

#[derive(Clone, Debug)]
pub(crate) struct Cylinder {
    center: Point,
    axis: Axis,
    length: f64,
    radius: f64,
}

impl Cylinder {
    #[must_use]
    pub(crate) fn new(center: Point, axis: Axis, length: f64, radius: f64) -> Self {
        assert_gt!(length, 0.0, "length must be a positive number");
        assert_gt!(radius, 0.0, "radius must be a positive number");
        Cylinder { center, axis, length, radius }
    }

    #[must_use]
    pub(crate) fn aabb(&self) -> Aabb {
        let half_length = self.length / 2.0;

        let (min, max) = match self.axis {
            Axis::X => {
                let min = Point::new(-half_length, -self.radius, -self.radius);
                let max = Point::new( half_length,  self.radius,  self.radius);
                (min, max)
            }
            Axis::Y => {
                let min = Point::new(-self.radius, -half_length, -self.radius);
                let max = Point::new( self.radius,  half_length,  self.radius);
                (min, max)
            }
            Axis::Z => {
                let min = Point::new(-self.radius, -self.radius, -half_length);
                let max = Point::new( self.radius,  self.radius,  half_length);
                (min, max)
            }
        };

        Aabb::from_points(min, max).translate(self.center.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::assert_abs_diff_eq;
    use rstest::rstest;

    #[test]
    fn test_cylinder_aabb_x_axis_at_origin() {
        let cylinder = Cylinder::new(Point::origin(), Axis::X, 8.0, 3.0);
        
        let aabb = cylinder.aabb();
        assert_abs_diff_eq!(aabb.min(), Point::new(-4.0, -3.0, -3.0));
        assert_abs_diff_eq!(aabb.max(), Point::new(4.0, 3.0, 3.0));
    }

    #[test]
    fn test_cylinder_aabb_y_axis_at_origin() {
        let cylinder = Cylinder::new(Point::origin(), Axis::Y, 10.0, 2.0);
        let aabb = cylinder.aabb();

        assert_abs_diff_eq!(aabb.min(), Point::new(-2.0, -5.0, -2.0));
        assert_abs_diff_eq!(aabb.max(), Point::new(2.0, 5.0, 2.0));
    }

    #[test]
    fn test_cylinder_aabb_z_axis_at_origin() {
        let cylinder = Cylinder::new(Point::origin(), Axis::Z, 6.0, 1.5);
        let aabb = cylinder.aabb();

        assert_abs_diff_eq!(aabb.min(), Point::new(-1.5, -1.5, -3.0));
        assert_abs_diff_eq!(aabb.max(), Point::new(1.5, 1.5, 3.0));
    }

    #[test]
    fn test_cylinder_aabb_x_axis_offset() {
        let center = Point::new(10.0, 5.0, -3.0);
        let cylinder = Cylinder::new(center, Axis::X, 8.0, 3.0);
        let aabb = cylinder.aabb();

        assert_abs_diff_eq!(aabb.min(), Point::new(6.0, 2.0, -6.0));
        assert_abs_diff_eq!(aabb.max(), Point::new(14.0, 8.0, 0.0));
    }

    #[test]
    fn test_cylinder_aabb_y_axis_offset() {
        let center = Point::new(2.0, 10.0, 4.0);
        let cylinder = Cylinder::new(center, Axis::Y, 10.0, 2.0);
        let aabb = cylinder.aabb();

        assert_abs_diff_eq!(aabb.min(), Point::new(0.0, 5.0, 2.0));
        assert_abs_diff_eq!(aabb.max(), Point::new(4.0, 15.0, 6.0));
    }

    #[test]
    fn test_cylinder_aabb_z_axis_offset() {
        let center = Point::new(-5.0, -2.0, 8.0);
        let cylinder = Cylinder::new(center, Axis::Z, 6.0, 1.5);
        let aabb = cylinder.aabb();

        assert_abs_diff_eq!(aabb.min(), Point::new(-6.5, -3.5, 5.0));
        assert_abs_diff_eq!(aabb.max(), Point::new(-3.5, -0.5, 11.0));
    }

    #[rstest]
    #[case( 0.0,  1.0)]
    #[case( 1.0,  0.0)]
    #[case(-1.0,  1.0)]
    #[case( 1.0, -1.0)]
    #[should_panic]
    fn test_cylinder_invalid_arguments(#[case] length: f64, #[case] radius: f64) {
        let _ = Cylinder::new(Point::origin(), Axis::X, length, radius);
    }

    #[test]
    fn test_cylinder_clone() {
        let center = Point::new(1.0, 2.0, 3.0);
        let original = Cylinder::new(center, Axis::Y, 10.0, 2.5);
        let clone = original.clone();

        assert_eq!(original.aabb(), clone.aabb());
    }
}