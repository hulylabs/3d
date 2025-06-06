use crate::bvh::proxy::{PrimitiveType, SceneObjectProxy};
use crate::geometry::aabb::Aabb;
use crate::objects::triangle::Triangle;

#[must_use]
pub(crate) fn proxy_of_triangle(index: usize, triangle: &Triangle, aabb_inflation_rate: f64) -> SceneObjectProxy {
    assert!(aabb_inflation_rate >= 0.0, "aabb_inflation is negative");
    let aabb = triangle.bounding_box();
    let aabb = aabb.max_extent_relative_inflate(aabb_inflation_rate);
    SceneObjectProxy::new(index, PrimitiveType::Triangle, aabb)
}

#[must_use]
pub(super) fn proxy_of_sdf(index: usize, aabb: Aabb) -> SceneObjectProxy {
    SceneObjectProxy::new(index, PrimitiveType::Sdf, aabb)
}

pub(crate) trait SceneObjects {
    fn make_proxies(&self, destination: &mut Vec<SceneObjectProxy>, aabb_inflation: f64);
}

impl SceneObjects for Vec<Triangle> {
    fn make_proxies(&self, destination: &mut Vec<SceneObjectProxy>, aabb_inflation_rate: f64) {
        assert!(aabb_inflation_rate >= 0.0, "aabb_inflation is negative");
        if self.is_empty() {
            return;
        }
        for (index, triangle) in self.iter().enumerate() {
            destination.push(proxy_of_triangle(index, triangle, aabb_inflation_rate));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::vertex::Vertex;
    use crate::objects::common_properties::Linkage;
    use crate::objects::material_index::MaterialIndex;
    use crate::utils::object_uid::ObjectUid;
    use cgmath::EuclideanSpace;
    use rstest::rstest;

    #[rstest]
    #[case(0.0)]
    #[case(1.0)]
    fn test_proxy_of_triangle_without_inflation(#[case] inflation: f64) {
        let expected_container_index = 17;
        let triangle = make_dummy_triangle();
        let actual_object = proxy_of_triangle(expected_container_index, &triangle, inflation);
        assert_eq!(actual_object.primitive_type(), PrimitiveType::Triangle); 
        assert_eq!(actual_object.host_container_index(), expected_container_index); 
        assert_eq!(actual_object.aabb(), triangle.bounding_box().max_extent_relative_inflate(inflation)); 
    }
    
    #[test]
    fn test_proxy_of_sdf() {
        let expected_container_index = 17;
        let actual_object = proxy_of_sdf(expected_container_index, Aabb::make_minimal());
        assert_eq!(actual_object.primitive_type(), PrimitiveType::Sdf); 
        assert_eq!(actual_object.host_container_index(), expected_container_index); 
    }

    #[must_use]
    fn make_dummy_triangle() -> Triangle {
        let dummy_vertex = Vertex::new(Point::origin(), Vector::unit_z());
        Triangle::new(dummy_vertex, dummy_vertex, dummy_vertex, Linkage::new(ObjectUid(0), MaterialIndex(0)))
    }
}
