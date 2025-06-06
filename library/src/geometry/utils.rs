use crate::geometry::alias::Vector;
use crate::geometry::axis::Axis;

pub(crate) trait Max {
    fn max_axis(self) -> Axis;
    fn max(self) -> f64;
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

    #[test]
    fn test_max() {
        assert_eq!(Vector::new(1.0, 2.0, 3.0).max(), 3.0);
        assert_eq!(Vector::new(1.0, 3.0, 2.0).max(), 3.0);
        assert_eq!(Vector::new(3.0, 1.0, 2.0).max(), 3.0);
        assert_eq!(Vector::new(3.0, 2.0, 1.0).max(), 3.0);
        assert_eq!(Vector::new(2.0, 3.0, 1.0).max(), 3.0);
        assert_eq!(Vector::new(2.0, 1.0, 3.0).max(), 3.0);
    }
}