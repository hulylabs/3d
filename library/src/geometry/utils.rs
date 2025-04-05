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
