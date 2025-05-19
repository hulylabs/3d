use crate::bvh::node::BvhNode;
use crate::objects::triangle::Triangle;
use std::cell::RefCell;
use std::rc::Rc;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::GpuSerializationSize;

#[must_use]
pub(crate) fn build_serialized_bvh(support: &mut[Triangle]) -> GpuReadySerializationBuffer {
    let root = BvhNode::make_for(support);

    BvhNode::populate_links(&mut root.borrow_mut(), None);

    let mut index = 0_usize;
    evaluate_serial_indices(Some(root.clone()), &mut index);

    let quartet_count = <BvhNode as GpuSerializationSize>::SERIALIZED_QUARTET_COUNT;
    let filler = 0.0;
    let mut serialized = GpuReadySerializationBuffer::make_filled(index, quartet_count, filler);
    serialize(Some(root.clone()), &mut serialized);

    serialized
}

fn serialize(candidate: Option<Rc<RefCell<BvhNode>>>, buffer: &mut GpuReadySerializationBuffer) {
    if candidate.is_none() {
        return;
    }

    // TODO: rewrite without recursion

    let anchor = candidate.unwrap();
    let node = anchor.borrow_mut();

    node.serialize_by_index_into(buffer);

    if node.left().is_some() {
        serialize(node.left().clone(), buffer);
    }
    if node.right().is_some() {
        serialize(node.right().clone(), buffer);
    }
}

fn evaluate_serial_indices(candidate: Option<Rc<RefCell<BvhNode>>>, index: &mut usize) {
    if candidate.is_none() {
        return;
    }

    // TODO: rewrite without recursion

    let anchor = candidate.unwrap();
    let mut node = anchor.borrow_mut();

    node.set_serial_index(*index);
    *index += 1;

    evaluate_serial_indices(node.left().clone(), index);
    evaluate_serial_indices(node.right().clone(), index);
}

#[cfg(test)]
mod tests {
    use bytemuck::checked::cast_slice;
    use crate::bvh::node::tests::make_triangle;
    use super::*;

    #[test]
    fn test_single_triangle() {
        let triangle = make_triangle([
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ]);
        let actual_serialized_bvh = build_serialized_bvh(&mut vec![triangle]);
        let expected_serialized_bvh = vec![0.0, 0.0, 0.0, -1.0, 1.0, 1.0, 1.0, 2.0, 0.0, 1.0, -1.0, 0.0];
        assert_eq!(cast_slice::<u8, f32>(actual_serialized_bvh.backend()), expected_serialized_bvh);
    }

    #[test]
    fn test_cube() {
        let mut cube_triangles = vec!
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

        let actual_serialized_bvh = build_serialized_bvh(&mut cube_triangles);

        let expected_serialized_bvh = vec!
        [
            -0.00005, -0.00005, -0.00005, 12.0, 1.00005, 2.00005, 3.00005, -1.0, -1.0, -1.0, -1.0, 2.0,
            0.0, 0.0, -0.00005, 7.0, 1.00005, 2.00005, 3.0, -1.0, -1.0, -1.0, 12.0, 2.0,
            0.0, 0.0, -0.00005, 6.0, 1.00005, 2.0, 3.0, -1.0, -1.0, -1.0, 7.0, 2.0,
            0.0, 0.0, -0.00005, 5.0, 1.0, 2.0, 0.00005, -1.0, -1.0, -1.0, 6.0, 1.0,
            0.0, 0.0, -0.00005, -1.0, 1.0, 2.0, 0.00005, 2.0, 0.0, 1.0, 5.0, 0.0,
            0.0, 0.0, -0.00005, -1.0, 1.0, 2.0, 0.00005, 2.0, 1.0, 1.0, 6.0, 0.0,
            0.99995, 0.0, 0.0, -1.0, 1.00005, 2.0, 3.0, 2.0, 2.0, 1.0, 7.0, 0.0,
            0.0, 0.0, 0.0, 11.0, 1.00005, 2.00005, 3.0, -1.0, -1.0, -1.0, 12.0, 2.0,
            0.0, 0.0, 0.0, 10.0, 1.00005, 2.00005, 3.0, -1.0, -1.0, -1.0, 11.0, 2.0,
            0.99995, 0.0, 0.0, -1.0, 1.00005, 2.0, 3.0, 2.0, 3.0, 1.0, 10.0, 0.0,
            0.0, 1.99995, 0.0, -1.0, 1.0, 2.00005, 3.0, 2.0, 4.0, 1.0, 11.0, 0.0,
            0.0, 1.99995, 0.0, -1.0, 1.0, 2.00005, 3.0, 2.0, 5.0, 1.0, 12.0, 0.0,
            -0.00005, -0.00005, 0.0, 18.0, 1.0, 2.0, 3.00005, -1.0, -1.0, -1.0, -1.0, 2.0,
            -0.00005, -0.00005, 0.0, 17.0, 1.0, 2.0, 3.0, -1.0, -1.0, -1.0, 18.0, 2.0,
            0.0, -0.00005, 0.0, 16.0, 1.0, 0.00005, 3.0, -1.0, -1.0, -1.0, 17.0, 2.0,
            0.0, -0.00005, 0.0, -1.0, 1.0, 0.00005, 3.0, 2.0, 6.0, 1.0, 16.0, 0.0,
            0.0, -0.00005, 0.0, -1.0, 1.0, 0.00005, 3.0, 2.0, 7.0, 1.0, 17.0, 0.0,
            -0.00005, 0.0, 0.0, -1.0, 0.00005, 2.0, 3.0, 2.0, 8.0, 1.0, 18.0, 0.0,
            -0.00005, 0.0, 0.0, 22.0, 1.0, 2.0, 3.00005, -1.0, -1.0, -1.0, -1.0, 2.0,
            -0.00005, 0.0, 0.0, 21.0, 1.0, 2.0, 3.00005, -1.0, -1.0, -1.0, 22.0, 2.0,
            -0.00005, 0.0, 0.0, -1.0, 0.00005, 2.0, 3.0, 2.0, 9.0, 1.0, 21.0, 0.0,
            0.0, 0.0, 2.99995, -1.0, 1.0, 2.0, 3.00005, 2.0, 10.0, 1.0, 22.0, 0.0,
            0.0, 0.0, 2.99995, -1.0, 1.0, 2.0, 3.00005, 2.0, 11.0, 1.0, -1.0, 0.0,
        ];
        assert_eq!(cast_slice::<u8, f32>(actual_serialized_bvh.backend()), expected_serialized_bvh);
    }
}