use crate::bvh::node::{get_bvh_node_children, BvhNode};
use crate::bvh::proxy::SceneObjectProxy;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::GpuSerializationSize;
use std::cell::RefCell;
use std::rc::Rc;
use crate::bvh::dfs::depth_first_search;

pub(crate) struct Bvh {
    root: Rc<RefCell<BvhNode>>,
    nodes_count: usize,
}

impl Bvh {
    #[must_use]
    pub(crate) fn root(&self) -> &Rc<RefCell<BvhNode>> {
        &self.root
    }
}

#[must_use]
pub(crate) fn build_bvh(support: &mut[SceneObjectProxy]) -> Bvh {
    let root = BvhNode::make_for(support);

    BvhNode::make_tree_threaded(root.clone());

    let mut nodes_count = 0_usize;
    evaluate_serial_indices(Some(root.clone()), &mut nodes_count);
    
    Bvh { root, nodes_count }
}

#[must_use]
pub(crate) fn build_serialized_bvh(support: &mut[SceneObjectProxy]) -> GpuReadySerializationBuffer {
    let bvh = build_bvh(support);

    let quartet_count = <BvhNode as GpuSerializationSize>::SERIALIZED_QUARTET_COUNT;
    let filler = 0.0;
    let mut serialized = GpuReadySerializationBuffer::make_filled(bvh.nodes_count, quartet_count, filler);
    serialize(Some(bvh.root.clone()), &mut serialized);

    serialized
}

fn serialize(candidate: Option<Rc<RefCell<BvhNode>>>, buffer: &mut GpuReadySerializationBuffer) {
    if candidate.is_none() {
        return;
    }
    depth_first_search(
        candidate.unwrap(),
        get_bvh_node_children,
        |node: &mut BvhNode, _next_right: Option<Rc<RefCell<BvhNode>>>| {
            node.serialize_by_index_into(buffer);
        }
    );
}

fn evaluate_serial_indices(candidate: Option<Rc<RefCell<BvhNode>>>, index: &mut usize) {
    if candidate.is_none() {
        return;
    }
    depth_first_search(
        candidate.unwrap(),
        get_bvh_node_children,
        |node: &mut BvhNode, _next_right: Option<Rc<RefCell<BvhNode>>>| {
            node.set_serial_index(*index);
            *index += 1;
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bvh::node::tests::make_triangle;
    use crate::bvh::proxy::PrimitiveType;
    use crate::container::bvh_proxies::{proxy_of_triangle, SceneObjects};
    use crate::serialization::gpu_ready_serialization_buffer::DEFAULT_PAD_VALUE;

    #[test]
    fn test_evaluate_serial_indices_none() {
        let mut nodes_count = 0;
        evaluate_serial_indices(None, &mut nodes_count);
        assert_eq!(nodes_count, 0);
    }

    #[test]
    fn test_evaluate_serial_indices_single_node() {
        let dummy_triangle = make_triangle([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let root = BvhNode::make_for(&mut vec![proxy_of_triangle(0, &dummy_triangle, 0.0)]);

        let mut nodes_count = 0;
        evaluate_serial_indices(Some(root.clone()), &mut nodes_count);

        assert_eq!(nodes_count, 1);
        assert_eq!(root.borrow().serial_index(), Some(0));
    }

    #[test]
    fn test_evaluate_serial_indices_root_with_two_leaves() {
        let triangle_one = make_triangle([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let triangle_two = make_triangle([2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0]);
        let root = BvhNode::make_for(&mut vec![
            proxy_of_triangle(0, &triangle_one, 0.0),
            proxy_of_triangle(1, &triangle_two, 0.0)
        ]);

        let mut nodes_count = 0;
        evaluate_serial_indices(Some(root.clone()), &mut nodes_count);
        
        assert_eq!(nodes_count, 3);

        assert_eq!(root.borrow().serial_index(), Some(0));

        let left_child = root.borrow().left().clone();
        assert!(left_child.is_some());
        assert_eq!(left_child.unwrap().borrow().serial_index(), Some(1));

        let right_child = root.borrow().right().clone();
        assert!(right_child.is_some());
        assert_eq!(right_child.unwrap().borrow().serial_index(), Some(2));
    }
    
    #[test]
    fn test_single_triangle() {
        let triangle = make_triangle([
            1.0, 0.0, 0.0,
            0.0, 2.0, 0.0,
            0.0, 0.0, 3.0,
        ]);
        let mut proxies = vec![proxy_of_triangle(0, &triangle, 0.0)];
        let actual_serialized_bvh = build_serialized_bvh(&mut proxies);
        let actual_serialized = actual_serialized_bvh.backend();

        assert_eq!(&actual_serialized[0 .. 4], &(0.0f32.to_ne_bytes()));
        assert_eq!(&actual_serialized[4 .. 8], &(0.0f32.to_ne_bytes()));
        assert_eq!(&actual_serialized[8 ..12], &(0.0f32.to_ne_bytes()));
        assert_eq!(&actual_serialized[12..16], &(0u32.to_ne_bytes()));

        assert_eq!(&actual_serialized[16..20], &(1.0f32.to_ne_bytes()));
        assert_eq!(&actual_serialized[20..24], &(2.0f32.to_ne_bytes()));
        assert_eq!(&actual_serialized[24..28], &(3.0f32.to_ne_bytes()));
        assert_eq!(&actual_serialized[28..32], &((PrimitiveType::Triangle as u32).to_ne_bytes()));

        assert_eq!(&actual_serialized[32..36], &((-1i32).to_ne_bytes()));
        assert_eq!(&actual_serialized[36..40], &(DEFAULT_PAD_VALUE.to_ne_bytes()));
        assert_eq!(&actual_serialized[40..44], &(DEFAULT_PAD_VALUE.to_ne_bytes()));
        assert_eq!(&actual_serialized[44..48], &(DEFAULT_PAD_VALUE.to_ne_bytes()));
    }

    #[test]
    fn test_cube() {
        let cube_triangles = vec!
        [
            make_triangle([0.0, 0.0, 0.0, /**/0.0, 2.0, 0.0, /**/1.0, 0.0, 0.0]),
            make_triangle([1.0, 0.0, 0.0, /**/0.0, 2.0, 0.0, /**/1.0, 2.0, 0.0]),

            make_triangle([1.0, 0.0, 0.0, /**/1.0, 2.0, 0.0, /**/1.0, 0.0, 3.0]),
            make_triangle([1.0, 2.0, 0.0, /**/1.0, 0.0, 3.0, /**/1.0, 2.0, 3.0]),

            make_triangle([1.0, 2.0, 0.0, /**/0.0, 2.0, 0.0, /**/1.0, 2.0, 3.0]),
            make_triangle([0.0, 2.0, 0.0, /**/0.0, 2.0, 3.0, /**/1.0, 2.0, 3.0]),

            make_triangle([1.0, 0.0, 3.0, /**/1.0, 2.0, 3.0, /**/0.0, 0.0, 3.0]),
            make_triangle([1.0, 2.0, 3.0, /**/0.0, 2.0, 3.0, /**/0.0, 0.0, 3.0]),

            make_triangle([1.0, 0.0, 3.0, /**/0.0, 0.0, 0.0, /**/1.0, 0.0, 0.0]),
            make_triangle([1.0, 0.0, 3.0, /**/0.0, 0.0, 3.0, /**/0.0, 0.0, 0.0]),

            make_triangle([0.0, 0.0, 0.0, /**/0.0, 0.0, 3.0, /**/0.0, 2.0, 3.0]),
            make_triangle([0.0, 0.0, 0.0, /**/0.0, 2.0, 3.0, /**/0.0, 2.0, 0.0]),
        ];
        
        let mut objects_to_tree: Vec<SceneObjectProxy> = Vec::with_capacity(cube_triangles.len());
        cube_triangles.make_proxies(&mut objects_to_tree, 0.0);

        let actual_serialized_bvh = build_serialized_bvh(&mut objects_to_tree);

        let expected_serialized_bvh: Vec<u8> = vec![23, 183, 81, 184, 23, 183, 81, 184, 23, 183, 81, 184, 0, 0, 0, 0, 163, 1, 128, 63, 210, 0, 0, 64, 210, 0, 64, 64, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 163, 1, 128, 63, 210, 0, 0, 64, 0, 0, 64, 64, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 163, 1, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 23, 183, 81, 56, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 23, 183, 81, 56, 2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 23, 183, 81, 184, 1, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 23, 183, 81, 56, 2, 0, 0, 0, 6, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 185, 252, 127, 63, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 163, 1, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64, 2, 0, 0, 0, 7, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 163, 1, 128, 63, 210, 0, 0, 64, 0, 0, 64, 64, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 163, 1, 128, 63, 210, 0, 0, 64, 0, 0, 64, 64, 0, 0, 0, 0, 11, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 185, 252, 127, 63, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 163, 1, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64, 2, 0, 0, 0, 10, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 93, 254, 255, 63, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 128, 63, 210, 0, 0, 64, 0, 0, 64, 64, 2, 0, 0, 0, 11, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 93, 254, 255, 63, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 128, 63, 210, 0, 0, 64, 0, 0, 64, 64, 2, 0, 0, 0, 12, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 23, 183, 81, 184, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 210, 0, 64, 64, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 23, 183, 81, 184, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64, 0, 0, 0, 0, 18, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 23, 183, 81, 56, 0, 0, 64, 64, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 128, 63, 23, 183, 81, 56, 0, 0, 64, 64, 2, 0, 0, 0, 16, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 23, 183, 81, 184, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 128, 63, 23, 183, 81, 56, 0, 0, 64, 64, 2, 0, 0, 0, 17, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 23, 183, 81, 56, 0, 0, 0, 64, 0, 0, 64, 64, 2, 0, 0, 0, 18, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 210, 0, 64, 64, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 210, 0, 64, 64, 0, 0, 0, 0, 22, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 23, 183, 81, 184, 0, 0, 0, 0, 0, 0, 0, 0, 11, 0, 0, 0, 23, 183, 81, 56, 0, 0, 0, 64, 0, 0, 64, 64, 2, 0, 0, 0, 21, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 46, 255, 63, 64, 6, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 210, 0, 64, 64, 2, 0, 0, 0, 22, 0, 0, 0, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 0, 0, 0, 0, 0, 0, 46, 255, 63, 64, 7, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 64, 210, 0, 64, 64, 2, 0, 0, 0, 255, 255, 255, 255, 0, 0, 128, 191, 0, 0, 128, 191, 0, 0, 128, 191];
        assert_eq!(actual_serialized_bvh.backend(), &expected_serialized_bvh, "serialized BVH for a cube does not match the reference");
    }
}