use crate::geometry::alias;

use alias::Point;
use alias::Vector;

pub(crate) struct AABB {
    min: Point,
    max: Point,
}

trait MinMax {
    fn component_wise_min(&self, other: &Point) -> Self;
    fn component_wise_max(&self, other: &Point) -> Self;
}

pub(crate) enum Axis
{
    X,
    Y,
    Z,

    Count,
}

impl MinMax for Point {

    #[must_use]
    fn component_wise_min(&self, other: &Point) -> Self {
        Point::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    #[must_use]
    fn component_wise_max(&self, other: &Point) -> Self {
        Point::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }
}

impl AABB {

    #[must_use]
    pub(crate) const fn new() -> Self {
        AABB {
            min: Point::new(f32::MAX, f32::MAX, f32::MAX),
            max: Point::new(f32::MIN, f32::MIN, f32::MIN),
        }
    }

    #[must_use]
    pub(crate) fn from_segment(a: Point, b: Point) -> Self {
        AABB {
            min: a.component_wise_min(&b),
            max: a.component_wise_max(&b),
        }
    }

    #[must_use]
    pub(crate) fn from_triangle(a: &Point, b: &Point, c: &Point) -> Self {
        AABB {
            min: a.component_wise_min(&b).component_wise_min(&c),
            max: a.component_wise_max(&b).component_wise_max(&c),
        }
    }

    #[must_use]
    pub(crate) fn merge(a: &AABB, b: &AABB) -> Self {
        AABB {
            min: a.min.component_wise_min(&b.min),
            max: a.max.component_wise_max(&b.max),
        }
    }

    const PAD_DELTA : f32 = 0.0001 / 2.0;

    #[must_use]
    pub(crate) fn pad(&self) -> Self{
        let mut result = AABB { min: self.min, max: self.max };
        for i in 0..Axis::Count as usize {
            if  result.max[i] - self.min[i] < AABB::PAD_DELTA {
                result.max[i] += AABB::PAD_DELTA;
                result.min[i] -= AABB::PAD_DELTA;
            }
        }
        result
    }

    #[must_use]
    pub(crate) fn extent(&self) -> Vector {
        self.max - self.min
    }

    #[must_use]
    pub(crate) fn centroid(&self, axis: Axis) -> f32 {
        let index = axis as usize;
        (self.min[index] + self.max[index]) / 2.0
    }

    #[must_use]
    pub(crate) fn half_surface_area(&self) -> f32 {
        let extent = self.extent();
        extent.x * extent.y + extent.y * extent.z + extent.z * extent.x
    }

    #[must_use]
    pub(crate) fn axis(&self, axis: Axis) -> (f32, f32) {
        let index = axis as usize;
        (self.min[index], self.max[index])
    }

    #[must_use]
    pub(crate) const fn min(&self) -> Point {
        self.min
    }

    #[must_use]
    pub(crate) const fn max(&self) -> Point {
        self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_wise_min() {
        let left = Point::new(7.0, 3.0, 5.0);
        let right = Point::new(6.0, 8.0, 4.0);
        assert_eq!(left.component_wise_min(&right), Point::new(6.0, 3.0, 4.0));
    }

    #[test]
    fn test_component_wise_max() {
        let left = Point::new(7.0, 3.0, 5.0);
        let right = Point::new(6.0, 8.0, 4.0);
        assert_eq!(left.component_wise_max(&right), Point::new(7.0, 8.0, 5.0));
    }

    #[test]
    fn test_aabb_new() {
        let system_under_test = AABB::new();
        assert_eq!(system_under_test.min, Point::new(f32::MAX, f32::MAX, f32::MAX));
        assert_eq!(system_under_test.max, Point::new(f32::MIN, f32::MIN, f32::MIN));
    }

    #[test]
    fn test_aabb_from_segment() {
        let start = Point::new(1.0, 4.0, 3.0);
        let end = Point::new(2.0, 2.0, 5.0);
        let system_under_test = AABB::from_segment(start, end);
        assert_eq!(system_under_test.min, Point::new(1.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_from_triangle() {
        let a = Point::new(1.0, 4.0, 3.0);
        let b = Point::new(2.0, 2.0, 5.0);
        let c = Point::new(0.0, 3.0, 4.0);
        let system_under_test = AABB::from_triangle(&a, &b, &c);
        assert_eq!(system_under_test.min, Point::new(0.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_merge() {
        let left = AABB::from_segment(
            Point::new(1.0, 4.0, 3.0),
            Point::new(2.0, 2.0, 5.0));

        let right = AABB::from_segment(
            Point::new(0.0, 3.0, 4.0),
            Point::new(3.0, 1.0, 6.0));

        let system_under_test = AABB::merge(&left, &right);

        assert_eq!(system_under_test.min, Point::new(0.0, 1.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(3.0, 4.0, 6.0));
    }

    #[test]
    fn test_aabb_pad_big_enough() {
        let system_under_test
            = AABB::from_segment(
                Point::new(1.0, 4.0, 3.0),
                Point::new(2.0, 2.0, 5.0))
            .pad();

        assert_eq!(system_under_test.min, Point::new(1.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_pad_small_enough() {
        let system_under_test
            = AABB::from_segment(
                Point::new(1.0, 4.0, 3.0),
                Point::new(2.0, 2.0, 5.0))
            .pad();

        assert_eq!(system_under_test.min, Point::new(1.0, 2.0, 3.0));
        assert_eq!(system_under_test.max, Point::new(2.0, 4.0, 5.0));
    }

    #[test]
    fn test_aabb_extent() {
        let system_under_test = AABB::from_segment(
            Point::new(1.0, 4.0, 3.0),
            Point::new(2.0, 2.0, 5.0));
        assert_eq!(system_under_test.extent(), Vector::new(1.0, 2.0, 2.0));
    }

    #[test]
    fn test_aabb_centroid() {
        let system_under_test = AABB::from_segment(
            Point::new(1.0, 4.0, 3.0),
            Point::new(2.0, 2.0, 5.0));
        assert_eq!(system_under_test.centroid(Axis::X), 1.5);
        assert_eq!(system_under_test.centroid(Axis::Y), 3.0);
        assert_eq!(system_under_test.centroid(Axis::Z), 4.0);
    }

    #[test]
    fn test_aabb_half_surface_area() {
        let system_under_test = AABB::from_segment(
            Point::new(1.0, 4.0, 3.0),
            Point::new(2.0, 2.0, 5.0));
        let expected_surface_area = 8.0;
        assert_eq!(system_under_test.half_surface_area(), expected_surface_area);
    }

    #[test]
    fn test_aabb_axis() {
        let system_under_test = AABB::from_segment(
            Point::new(1.0, 4.0, 3.0),
            Point::new(2.0, 2.0, 5.0));
        assert_eq!(system_under_test.axis(Axis::X), (1.0, 2.0));
        assert_eq!(system_under_test.axis(Axis::Y), (2.0, 4.0));
        assert_eq!(system_under_test.axis(Axis::Z), (3.0, 5.0));
    }
}