use crate::geometry::alias;
use crate::geometry::axis::Axis;
use crate::geometry::epsilon::DEFAULT_EPSILON_F64;
use alias::Point;
use alias::Vector;
use cgmath::AbsDiffEq;
use strum::EnumCount;

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Aabb {
    min: Point,
    max: Point,
}

impl Aabb {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Aabb {
            min: Point::new(f64::MAX, f64::MAX, f64::MAX),
            max: Point::new(f64::MIN, f64::MIN, f64::MIN),
        }
    }

    #[must_use]
    pub(crate) fn from_triangle(a: Point, b: Point, c: Point) -> Self {
        Aabb {
            min: a.component_wise_min(b).component_wise_min(c),
            max: a.component_wise_max(b).component_wise_max(c),
        }
    }

    #[must_use]
    pub(crate) fn merge(a: Aabb, b: Aabb) -> Self {
        Aabb {
            min: a.min.component_wise_min(b.min),
            max: a.max.component_wise_max(b.max),
        }
    }

    const PAD_DELTA: f64 = 0.0001 / 2.0;

    #[must_use]
    pub(crate) fn pad(self) -> Self {
        let mut result = Aabb { min: self.min, max: self.max };
        for i in 0..Axis::COUNT {
            if result.max[i] - self.min[i] < Aabb::PAD_DELTA {
                result.max[i] += Aabb::PAD_DELTA;
                result.min[i] -= Aabb::PAD_DELTA;
            }
        }
        result
    }

    #[must_use]
    pub(crate) fn extent(self) -> Vector {
        self.max - self.min
    }

    #[must_use]
    pub(crate) fn axis(self, axis: Axis) -> (f64, f64) {
        let index = axis as usize;
        (self.min[index], self.max[index])
    }

    #[must_use]
    pub(crate) const fn min(self) -> Point {
        self.min
    }

    #[must_use]
    pub(crate) const fn max(self) -> Point {
        self.max
    }
}

trait MinMax {
    fn component_wise_min(self, other: Point) -> Self;
    fn component_wise_max(self, other: Point) -> Self;
}

impl MinMax for Point {
    #[must_use]
    fn component_wise_min(self, other: Point) -> Self {
        Point::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z))
    }
    #[must_use]
    fn component_wise_max(self, other: Point) -> Self {
        Point::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z))
    }
}

impl AbsDiffEq for Aabb {
    type Epsilon = f64;

    #[must_use]
    fn default_epsilon() -> Self::Epsilon {
        DEFAULT_EPSILON_F64
    }

    #[must_use]
    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Point::abs_diff_eq(&self.min, &other.min, epsilon) && Point::abs_diff_eq(&self.max, &other.max, epsilon)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[must_use]
    fn from_segment(a: Point, b: Point) -> Aabb {
        Aabb {
            min: a.component_wise_min(b),
            max: a.component_wise_max(b),
        }
    }

    #[test]
    fn test_component_wise_min() {
        let left = Point::new(7.0, 3.0, 5.0);
        let right = Point::new(6.0, 8.0, 4.0);
        assert_eq!(left.component_wise_min(right), Point::new(6.0, 3.0, 4.0));
    }

    #[test]
    fn test_component_wise_max() {
        let left = Point::new(7.0, 3.0, 5.0);
        let right = Point::new(6.0, 8.0, 4.0);
        assert_eq!(left.component_wise_max(right), Point::new(7.0, 8.0, 5.0));
    }

    #[test]
    fn test_aabb_new() {
        let system_under_test = Aabb::new();
        assert_eq!(system_under_test.min, Point::new(f64::MAX, f64::MAX, f64::MAX));
        assert_eq!(system_under_test.max, Point::new(f64::MIN, f64::MIN, f64::MIN));
    }

    #[test]
    fn test_aabb_from_segment() {
        let start = Point::new(1.0, 4.0, 3.0);
        let end = Point::new(2.0, 2.0, 5.0);
        let system_under_test = from_segment(start, end);
        assert_eq!(system_under_test.min, Point::new(1.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_from_triangle() {
        let a = Point::new(1.0, 4.0, 3.0);
        let b = Point::new(2.0, 2.0, 5.0);
        let c = Point::new(0.0, 3.0, 4.0);
        let system_under_test = Aabb::from_triangle(a, b, c);
        assert_eq!(system_under_test.min, Point::new(0.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_merge() {
        let left = from_segment(Point::new(1.0, 4.0, 3.0), Point::new(2.0, 2.0, 5.0));

        let right = from_segment(Point::new(0.0, 3.0, 4.0), Point::new(3.0, 1.0, 6.0));

        let system_under_test = Aabb::merge(left, right);

        assert_eq!(system_under_test.min, Point::new(0.0, 1.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(3.0, 4.0, 6.0));
    }

    #[test]
    fn test_aabb_pad_big_enough() {
        let system_under_test = from_segment(Point::new(1.0, 4.0, 3.0), Point::new(2.0, 2.0, 5.0)).pad();

        assert_eq!(system_under_test.min, Point::new(1.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_pad_small_enough() {
        let system_under_test = from_segment(Point::new(1.0, 4.0, 5.0), Point::new(1.0, 4.0, 5.0)).pad();

        assert_eq!(system_under_test.min, Point::new(1.0 - Aabb::PAD_DELTA, 4.0 - Aabb::PAD_DELTA, 5.0 - Aabb::PAD_DELTA));
        assert_eq!(system_under_test.max, Point::new(1.0 + Aabb::PAD_DELTA, 4.0 + Aabb::PAD_DELTA, 5.0 + Aabb::PAD_DELTA));
    }

    #[test]
    fn test_aabb_extent() {
        let system_under_test = from_segment(Point::new(1.0, 4.0, 3.0), Point::new(2.0, 2.0, 5.0));
        assert_eq!(system_under_test.extent(), Vector::new(1.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_axis() {
        let system_under_test = from_segment(Point::new(1.0, 4.0, 3.0), Point::new(2.0, 2.0, 5.0));
        assert_eq!(system_under_test.axis(Axis::X), (1.0, 2.0));
        assert_eq!(system_under_test.axis(Axis::Y), (2.0, 4.0));
        assert_eq!(system_under_test.axis(Axis::Z), (3.0, 5.0));
    }
}
