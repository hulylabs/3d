use cgmath::Vector2;
use float_cmp::approx_eq;
use crate::geometry::alias::{Point, Vector};
use crate::geometry::axis::Axis;
use crate::geometry::transform::Affine;

pub(crate) trait Max {
    fn max_axis(self) -> Axis;

    #[cfg(test)]
    fn max(self) -> f64;
}

#[must_use]
pub(crate) fn is_affine(matrix: &Affine) -> bool {
    approx_eq!(f64, matrix.x.w, 0.0, epsilon = f64::EPSILON) &&
    approx_eq!(f64, matrix.y.w, 0.0, epsilon = f64::EPSILON) && 
    approx_eq!(f64, matrix.z.w, 0.0, epsilon = f64::EPSILON) && 
    approx_eq!(f64, matrix.w.w, 1.0, epsilon = f64::EPSILON)
}

impl Max for Vector {
    #[must_use]
    fn max_axis(self) -> Axis {
        let mut axis = Axis::X;
        if self[Axis::Y as usize] > self[axis as usize] {
            axis = Axis::Y;
        }
        if self[Axis::Z as usize] > self[axis as usize] {
            axis = Axis::Z;
        }
        axis
    }

    #[cfg(test)]
    #[must_use]
    fn max(self) -> f64 {
        if self.x > self.y {
            if self.x > self.z {
                self.x
            } else {
                self.z
            }
        } else {
            if self.y > self.z {
                self.y
            } else {
                self.z
            }
        }
    }
}

#[must_use]
pub(crate) fn debug_format_human_readable_point(point: Point) -> String {
    const MAX_CHARS_TO_OUTPUT: usize = 5;
    let format_coord = |coord: f64| -> String {
        let s = format!("{:.3}", coord);
        if s.len() <= MAX_CHARS_TO_OUTPUT {
            s
        } else {
            s.chars().take(MAX_CHARS_TO_OUTPUT).collect()
        }
    };
    format!("{},{},{}", format_coord(point.x), format_coord(point.y), format_coord(point.z))
}

#[must_use]
pub(crate) fn exclude_axis(victim: Vector, exclusion: Axis) -> Vector2<f64> {
    let keep_a = exclusion.next();
    let keep_b = keep_a.next();
    Vector2::<f64>::new(victim[keep_a.as_index()], victim[keep_b.as_index()], )
}

#[cfg(test)]
mod tests {
    use cgmath::{Array, SquareMatrix};
    use super::*;

    #[test]
    fn test_max_axis_x() {
        let system_under_test = Vector::new(10.0, 5.0, 1.0);
        let actual_max_axis = system_under_test.max_axis();
        assert_eq!(actual_max_axis, Axis::X);
    }

    #[test]
    fn test_max_axis_y() {
        let system_under_test = Vector::new(5.0, 15.0, 10.0);
        let actual_max_axis = system_under_test.max_axis();
        assert_eq!(actual_max_axis, Axis::Y);
    }

    #[test]
    fn test_max_axis_z() {
        let system_under_test = Vector::new(1.0, 5.0, 20.0);
        let actual_max_axis = system_under_test.max_axis();
        assert_eq!(actual_max_axis, Axis::Z);
    }

    #[test]
    fn test_max_axis_all_equal() {
        let system_under_test = Vector::new(5.0, 5.0, 5.0);
        let actual_max_axis = system_under_test.max_axis();
        assert_eq!(actual_max_axis, Axis::X);
    }

    #[test]
    fn test_max() {
        assert_eq!(Vector::new(1.0, 2.0, 3.0).max(), 3.0);
        assert_eq!(Vector::new(1.0, 3.0, 2.0).max(), 3.0);
        assert_eq!(Vector::new(3.0, 1.0, 2.0).max(), 3.0);
        assert_eq!(Vector::new(3.0, 2.0, 1.0).max(), 3.0);
        assert_eq!(Vector::new(2.0, 3.0, 1.0).max(), 3.0);
        assert_eq!(Vector::new(2.0, 1.0, 3.0).max(), 3.0);
    }

    #[test]
    fn test_exclude_axis() {
        let victim = Vector::new(1.0, 2.0, 3.0);
        assert_eq!(exclude_axis(victim, Axis::X), Vector2::<f64>::new(victim[1], victim[2]));
        assert_eq!(exclude_axis(victim, Axis::Y), Vector2::<f64>::new(victim[2], victim[0]));
        assert_eq!(exclude_axis(victim, Axis::Z), Vector2::<f64>::new(victim[0], victim[1]));
    }

    #[test]
    fn test_debug_format_human_readable_point() {
        let point = Point::new(-1.0, 2.12345, -3.12345);
        let actual_format = debug_format_human_readable_point(point);
        assert_eq!(actual_format, "-1.00,2.123,-3.12");
    }

    #[test]
    fn test_is_affine() {
        let mut probe = Affine::identity();
        assert!(is_affine(&probe));
        
        probe.x.w = 0.0000001;
        assert_eq!(is_affine(&probe), false);

        probe = Affine::from_translation(Vector::from_value(7.9));
        assert!(is_affine(&probe));

        probe.w.w = 1.0000001;
        assert_eq!(is_affine(&probe), false);
    }
}