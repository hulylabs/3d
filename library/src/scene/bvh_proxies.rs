use crate::bvh::proxy::{PrimitiveType, SceneObjectProxy};
use crate::geometry::aabb::Aabb;
use crate::objects::triangle::Triangle;

#[must_use]
pub(crate) fn proxy_of_triangle(index: usize, triangle: &Triangle) -> SceneObjectProxy {
    SceneObjectProxy::new(index, PrimitiveType::Triangle, triangle.bounding_box())
}

#[must_use]
pub(super) fn proxy_of_sdf(index: usize, aabb: Aabb) -> SceneObjectProxy {
    SceneObjectProxy::new(index, PrimitiveType::Sdf, aabb)
}

pub(crate) trait SceneObjects {
    fn make_proxies(&self, destination: &mut Vec<SceneObjectProxy>);
}

impl SceneObjects for Vec<Triangle> {
    fn make_proxies(&self, destination: &mut Vec<SceneObjectProxy>) {
        for (index, triangle) in self.iter().enumerate() {
            destination.push(proxy_of_triangle(index, triangle));
        }
    }
}