use crate::bvh::proxy::SceneObjectProxy;
use crate::objects::triangle::Triangle;

#[must_use]
pub(crate) fn proxy_of_triangle(index: usize, triangle: &Triangle) -> SceneObjectProxy {
    SceneObjectProxy::new(index, triangle.bounding_box())
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