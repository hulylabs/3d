﻿use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use strum::EnumCount;
use crate::bvh::dfs::depth_first_search;
use crate::bvh::proxy::{PrimitiveType, SceneObjectProxy};
use crate::geometry::utils::Max;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::serialization::serializable_for_gpu::GpuSerializationSize;

struct BvhNodeContent {
    primitive_index: usize,
    primitive_type: PrimitiveType,
}

impl BvhNodeContent {
    #[must_use]
    fn new(primitive_index: usize, primitive_type: PrimitiveType) -> Self {
        Self {
            primitive_index,
            primitive_type,
        }
    }
    
    #[must_use]
    fn primitive_index(&self) -> usize {
        self.primitive_index
    }

    #[must_use]
    fn primitive_type(&self) -> PrimitiveType {
        self.primitive_type
    }
}

type BvhNodeReference = Option<Rc<RefCell<BvhNode>>>;

pub(crate) struct BvhNode {
    left: BvhNodeReference,
    right: BvhNodeReference,
    bounding_box: Aabb,
    content: Option<BvhNodeContent>,
    serial_index: Option<usize>,
    hit_node: BvhNodeReference,
    miss_node: BvhNodeReference,
    right_offset: BvhNodeReference,
    axis: Axis,
}

#[must_use]
pub(super) fn get_bvh_node_children(node: &BvhNode) -> (BvhNodeReference, BvhNodeReference) {
    (node.left().clone(), node.right().clone())
}

impl GpuSerializationSize for BvhNode {
    const SERIALIZED_QUARTET_COUNT: usize = 3;
}

impl BvhNode {
    #[must_use]
    fn new() -> Self {
        Self {
            left: None,
            right: None,
            bounding_box: Aabb::make_null(),
            content: None,
            serial_index: None,
            hit_node: None,
            miss_node: None,
            right_offset: None,
            axis: Axis::X,
        }
    }

    const GPU_NULL_REFERENCE_MARKER: i32 = -1;

    #[must_use]
    fn index_of_or_null(node: &BvhNodeReference) -> i32 {
        if let Some(node) = &node {
            match node.borrow().serial_index {
                Some(index) => index as i32,
                None => BvhNode::GPU_NULL_REFERENCE_MARKER,
            }
        } else {
            BvhNode::GPU_NULL_REFERENCE_MARKER
        }
    }
    
    #[must_use]
    pub(super) fn miss_node_index_or_null(&self) -> i32 {
        BvhNode::index_of_or_null(&self.miss_node)
    }

    #[must_use]
    pub(crate) fn make_for(support: &mut [SceneObjectProxy]) -> Rc<RefCell<BvhNode>> {
        if support.is_empty() {
            return Rc::new(RefCell::new(BvhNode::new()));
        }
        BvhNode::build_hierarchy(support)
    }

    pub(super) fn set_serial_index(&mut self, serial_index: usize) {
        self.serial_index = Some(serial_index);
    }

    #[must_use]
    pub(super) fn left(&self) -> &BvhNodeReference {
        &self.left
    }

    #[must_use]
    pub(super) fn right(&self) -> &BvhNodeReference {
        &self.right
    }

    #[must_use]
    pub(super) fn serial_index(&self) -> Option<usize> {
        self.serial_index
    }
    
    #[must_use]
    pub(super) fn content_type(&self) -> Option<PrimitiveType> {
        self.content.as_ref().map(|content| content.primitive_type)
    }

    #[must_use]
    pub(super) fn content_index(&self) -> Option<usize> {
        self.content.as_ref().map(|content| content.primitive_index)
    }

    #[must_use]
    pub(super) fn aabb(&self) -> &Aabb {
        &self.bounding_box
    }

    #[must_use]
    fn build_hierarchy(support: &mut [SceneObjectProxy]) -> Rc<RefCell<BvhNode>> {
        assert!(!support.is_empty());
        
        struct StackItem {
            start: usize,
            end: usize,
            parent: BvhNodeReference,
            is_left: bool,
        }

        let mut stack = Vec::<StackItem>::new();
        let mut root: BvhNodeReference = None;
        stack.push(StackItem {
            start: 0,
            end: support.len() - 1,
            parent: None,
            is_left: false,
        });

        while let Some(StackItem { start, end, parent, is_left }) = stack.pop() {
            let mut node = BvhNode::new();
            for i in start..=end {
                node.bounding_box = Aabb::make_union(node.bounding_box, support[i].aabb());
            }

            let span = end - start;
            let current_node = Rc::new(RefCell::new(node));

            if let Some(parent_node) = parent.clone() {
                if is_left {
                    parent_node.borrow_mut().left = Some(current_node.clone());
                } else {
                    parent_node.borrow_mut().right = Some(current_node.clone());
                }
            } else {
                root = Some(current_node.clone());
            }

            if span > 0 {
                let axis = current_node.borrow().bounding_box.extent().max_axis();
                let comparator = BvhNode::COMPARATORS[axis as usize];

                let mut subarray = support[start..=end].to_vec();
                subarray.sort_by(comparator);
                for (i, object) in subarray.iter().enumerate() {
                    support[start + i] = *object;
                }

                let middle = start + span / 2;
                
                stack.push(StackItem {
                    start: middle + 1,
                    end,
                    parent: Some(current_node.clone()),
                    is_left: false,
                });

                stack.push(StackItem {
                    start,
                    end: middle,
                    parent: Some(current_node.clone()),
                    is_left: true,
                });

                current_node.borrow_mut().axis = axis;
            } else {
                let proxy = &support[start];
                let object_index = proxy.host_container_index();
                let object_type = proxy.primitive_type();
                current_node.borrow_mut().content = Some(BvhNodeContent::new(object_index, object_type));
            }
        }

        assert!(root.is_some(), "at least one node must have created a tree");
        root.unwrap()
    }

    // "Implementing a practical rendering system using GLSL" by Toshiya Hachisuka
    pub(crate) fn make_tree_threaded(bvh: Rc<RefCell<BvhNode>>) {
        depth_first_search(
            bvh,
            get_bvh_node_children,
            |node: &mut BvhNode, next_right: BvhNodeReference| {
                if node.content.is_none() {
                    node.hit_node = node.left.clone();
                    node.miss_node = next_right.clone();
                    node.right_offset = node.right.clone();
                } else {
                    node.hit_node = next_right.clone();
                    node.miss_node = next_right.clone();
                }
            }
        );
    }

    #[must_use]
    fn box_compare(left: &SceneObjectProxy, right: &SceneObjectProxy, axis: Axis) -> Ordering {
        let left_axis_value = left.aabb().axis(axis).0;
        let right_axis_value = right.aabb().axis(axis).0;

        left_axis_value.partial_cmp(&right_axis_value).unwrap_or(Ordering::Equal)
    }

    #[must_use]
    fn box_x_compare(left: &SceneObjectProxy, right: &SceneObjectProxy) -> Ordering {
        BvhNode::box_compare(left, right, Axis::X)
    }

    #[must_use]
    fn box_y_compare(left: &SceneObjectProxy, right: &SceneObjectProxy) -> Ordering {
        BvhNode::box_compare(left, right, Axis::Y)
    }

    #[must_use]
    fn box_z_compare(left: &SceneObjectProxy, right: &SceneObjectProxy) -> Ordering {
        BvhNode::box_compare(left, right, Axis::Z)
    }

    const COMPARATORS: [fn(&SceneObjectProxy, &SceneObjectProxy) -> Ordering; Axis::COUNT] = [
        BvhNode::box_x_compare,
        BvhNode::box_y_compare,
        BvhNode::box_z_compare,
    ];

    pub(super) fn serialize_by_index_into(&self, container: &mut GpuReadySerializationBuffer) {
        assert!(self.serial_index.is_some(), "index was not set");
        debug_assert!(container.fully_written(), "buffer underflow");

        let index = self.serial_index().unwrap();
        
        let primitive_index: u32;
        let primitive_type: u32;
        match self.content.as_ref() {
            Some(content) => {
                primitive_index = content.primitive_index() as u32;
                primitive_type = content.primitive_type() as u32;
            }
            None => {
                primitive_index = 0;
                primitive_type = PrimitiveType::Null as u32;
            },
        }

        container.write_object(index, |writer|{
            
            writer.write_quartet(|writer| {
                writer.write_float_64(self.bounding_box.min().x);
                writer.write_float_64(self.bounding_box.min().y);
                writer.write_float_64(self.bounding_box.min().z);
                writer.write_unsigned(primitive_index);
            });

            writer.write_quartet(|writer| {
                writer.write_float_64(self.bounding_box.max().x);
                writer.write_float_64(self.bounding_box.max().y);
                writer.write_float_64(self.bounding_box.max().z);
                writer.write_unsigned(primitive_type);
            });

            writer.write_quartet(|writer| {
                writer.write_signed(self.miss_node_index_or_null());
            });
            
        });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::epsilon::DEFAULT_EPSILON_F64;
    use crate::geometry::fundamental_constants::VERTICES_IN_TRIANGLE;
    use crate::geometry::vertex::Vertex;
    use cgmath::{Zero, assert_abs_diff_eq};
    use strum::EnumCount;
    use crate::container::bvh_proxies::proxy_of_triangle;
    use crate::objects::common_properties::{Linkage, ObjectUid};
    use crate::objects::material_index::MaterialIndex;
    use crate::objects::triangle::Triangle;

    #[must_use]
    pub(crate) fn make_triangle(vertex_data: [f64; VERTICES_IN_TRIANGLE * Axis::COUNT]) -> Triangle {
        Triangle::new(
            Vertex::new(Point::new(vertex_data[0], vertex_data[1], vertex_data[2]), Vector::zero()),
            Vertex::new(Point::new(vertex_data[3], vertex_data[4], vertex_data[5]), Vector::zero()),
            Vertex::new(Point::new(vertex_data[6], vertex_data[7], vertex_data[8]), Vector::zero()),
            Linkage::new(ObjectUid(0), MaterialIndex(0)),
        )
    }

    #[test]
    fn test_empty_support() {
        let ware = BvhNode::make_for(&mut vec![]);

        let system_under_test = ware.borrow();

        assert!(system_under_test.left.is_none());
        assert!(system_under_test.right.is_none());
        assert_abs_diff_eq!(system_under_test.bounding_box, Aabb::make_null(), epsilon = DEFAULT_EPSILON_F64);
        assert!(system_under_test.content.is_none());
        assert!(system_under_test.hit_node.is_none());
        assert!(system_under_test.miss_node.is_none());
        assert!(system_under_test.right_offset.is_none());
        assert_eq!(system_under_test.axis, Axis::X);
    }

    #[test]
    fn test_single_triangle_support() {
        let triangle = make_triangle([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let ware = BvhNode::make_for(&mut vec![proxy_of_triangle(0, &triangle, 0.0)]);

        let system_under_test = ware.borrow();

        assert!(system_under_test.left.is_none());
        assert!(system_under_test.right.is_none());
        assert_abs_diff_eq!(system_under_test.bounding_box, triangle.bounding_box(), epsilon = DEFAULT_EPSILON_F64);
        assert_eq!(system_under_test.content.as_ref().unwrap().primitive_index(), 0);
        assert!(system_under_test.hit_node.is_none());
        assert!(system_under_test.miss_node.is_none());
        assert!(system_under_test.right_offset.is_none());
        assert_eq!(system_under_test.axis, Axis::X);
    }

    fn test_two_triangle_support(axis_offset: Vector, expected_axis: Axis) {
        let left = make_triangle([
            1.0 + axis_offset.x, 0.0 + axis_offset.y, 0.0 + axis_offset.z,
            0.0 + axis_offset.x, 1.0 + axis_offset.y, 0.0 + axis_offset.z,
            0.0 + axis_offset.x, 0.0 + axis_offset.y, 1.0 + axis_offset.z,
        ]);
        let right = make_triangle([
            1.0 - axis_offset.x, 0.0 - axis_offset.y, 0.0 - axis_offset.z,
            0.0 - axis_offset.x, 1.0 - axis_offset.y, 0.0 - axis_offset.z,
            0.0 - axis_offset.x, 0.0 - axis_offset.y, 1.0 - axis_offset.z,
        ]);
        let ware = BvhNode::make_for(&mut vec![proxy_of_triangle(0, &left, 0.0), proxy_of_triangle(0, &right, 0.0)]);

        let system_under_test = ware.borrow();

        assert_abs_diff_eq!(system_under_test.bounding_box, Aabb::make_union(left.bounding_box(), right.bounding_box()), epsilon = DEFAULT_EPSILON_F64);
        assert!(system_under_test.content.is_none());
        assert!(system_under_test.hit_node.is_none());
        assert!(system_under_test.miss_node.is_none());
        assert!(system_under_test.right_offset.is_none());
        assert_eq!(system_under_test.axis, expected_axis);
    }

    #[test]
    fn test_two_along_x_triangle_support() {
        test_two_triangle_support(Vector::unit_x(), Axis::X);
    }

    #[test]
    fn test_two_along_y_triangle_support() {
        test_two_triangle_support(Vector::unit_y(), Axis::Y);
    }

    #[test]
    fn test_two_along_z_triangle_support() {
        test_two_triangle_support(Vector::unit_z(), Axis::Z);
    }

    #[test]
    fn test_index_of_or_null_with_none() {
        assert_eq!(BvhNode::index_of_or_null(&None), BvhNode::GPU_NULL_REFERENCE_MARKER);
    }

    #[test]
    fn test_index_of_or_null_with_node() {
        let mut victim = BvhNode::new();
        let expected_index = 13_usize;
        victim.set_serial_index(expected_index);
        assert_eq!(BvhNode::index_of_or_null(&Some(Rc::new(RefCell::new(victim)))), expected_index as i32);
    }

    #[test]
    fn test_set_serial_index() {
        let mut victim = BvhNode::new();
        assert_eq!(victim.serial_index(), None);

        let expected_index = 13_usize;
        victim.set_serial_index(expected_index);
        assert_eq!(victim.serial_index().unwrap(), expected_index);
    }

    #[test]
    fn test_left() {
        let node = BvhNode::new();
        assert!(node.left().is_none(), "Expected left to be None");
    }

    #[test]
        fn test_right() {
        let node = BvhNode::new();
        assert!(node.right().is_none(), "Expected right to be None");
    }
    
    #[test]
    fn test_miss_node_index_or_null() {
        let mut node = BvhNode::new();
        assert_eq!(node.miss_node_index_or_null(), BvhNode::GPU_NULL_REFERENCE_MARKER);

        let miss_node = Rc::new(RefCell::new(BvhNode::new()));
        let expected_miss_node_index = 3;
        miss_node.borrow_mut().set_serial_index(expected_miss_node_index);
        node.miss_node = Some(Rc::clone(&miss_node));

        assert_eq!(node.miss_node_index_or_null(), expected_miss_node_index as i32);
    }

    #[test]
    fn test_make_for_empty_support() {
        let mut support = Vec::new();
        let node = BvhNode::make_for(&mut support);

        assert!(node.borrow().left.is_none());
        assert!(node.borrow().right.is_none());
    }
}
