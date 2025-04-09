use crate::geometry::alias::Vector;
use crate::geometry::axis::Axis;

pub(crate) trait MaxAxis {
    fn max_axis(self) -> Axis;
}

impl MaxAxis for Vector {
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
}

#[cfg(test)]
mod tests {
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
}