use crate::geometry::aabb::Aabb;
use crate::sdf::framework::sdf_base::Sdf;
use std::rc::Rc;

#[must_use]
pub(in crate::sdf) fn intersection_aabb(left: Rc<dyn Sdf>, right: Rc<dyn Sdf>,) -> Aabb {
    let left = left.aabb();
    let right = right.aabb();

    if let Some(intersection) = Aabb::make_intersection(left, right) {
        intersection
    } else {
        Aabb::make_minimal()
    }
}