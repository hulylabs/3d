use cgmath::InnerSpace;
use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use crate::geometry::cylinder::Cylinder;
use crate::geometry::utils::exclude_axis;

#[must_use]
pub(super) fn circumscribed_cylinder(aabb: &Aabb, axis: Axis) -> Cylinder {
    let extent = aabb.extent();
    let length = extent[axis.as_index()];
    let radius = exclude_axis(extent, axis) / 2.0;

    Cylinder::new(aabb.center(), axis, length, radius.magnitude())
}