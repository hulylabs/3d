use std::rc::Rc;
use crate::geometry::aabb::Aabb;
use crate::sdf::sdf_base::Sdf;

#[must_use]
pub(super) fn intersection_aabb(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>,) -> Aabb {
    let left = left.aabb();
    let right = right.aabb();

    if let Some(intersection) = Aabb::make_intersection(left, right) {
        intersection
    } else {
        Aabb::make_minimal()
    }
}