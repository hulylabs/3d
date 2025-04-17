use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use crate::objects::triangle::Triangle;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::DerefMut;
use std::rc::Rc;
use strum::EnumCount;
use crate::geometry::utils::MaxAxis;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

struct BvhNodeContent {
    start_triangle_index: usize,
    triangles_count: usize,
}

impl BvhNodeContent {
    #[must_use]
    fn new(start_triangle_index: usize, triangles_count: usize) -> Self {
        assert!(triangles_count > 0);
        BvhNodeContent {
            start_triangle_index,
            triangles_count,
        }
    }
    #[must_use]
    fn start_triangle_index(&self) -> usize {
        self.start_triangle_index
    }
    #[must_use]
    fn triangles_count(&self) -> usize {
        self.triangles_count
    }
}

pub(crate) struct BvhNode {
    left: Option<Rc<RefCell<BvhNode>>>,
    right: Option<Rc<RefCell<BvhNode>>>,
    bounding_box: Aabb,
    content: Option<BvhNodeContent>,
    serial_index: Option<usize>,
    hit_node: Option<Rc<RefCell<BvhNode>>>,
    miss_node: Option<Rc<RefCell<BvhNode>>>,
    right_offset: Option<Rc<RefCell<BvhNode>>>,
    axis: Axis,
}

impl BvhNode {
    #[must_use]
    fn new() -> Self {
        Self {
            left: None,
            right: None,
            bounding_box: Aabb::new(),
            content: None,
            serial_index: None,
            hit_node: None, // TODO: not used? check the algorithm
            miss_node: None,
            right_offset: None,
            axis: Axis::X,
        }
    }

    const GPU_NULL_REFERENCE_MARKER: f64 = -1.0;

    #[must_use]
    fn index_of_or_null(node: &Option<Rc<RefCell<BvhNode>>>) -> f64 {
        if let Some(node) = &node {
            match node.borrow().serial_index {
                Some(index) => index as f64,
                None => BvhNode::GPU_NULL_REFERENCE_MARKER,
            }
        } else {
            BvhNode::GPU_NULL_REFERENCE_MARKER
        }
    }

    #[must_use]
    fn right_offset_index_or_null(&self) -> f64 {
        BvhNode::index_of_or_null(&self.right_offset)
    }

    #[must_use]
    fn miss_node_index_or_null(&self) -> f64 {
        BvhNode::index_of_or_null(&self.miss_node)
    }

    #[must_use]
    pub(super) fn make_for(support: &mut Vec<Triangle>) -> Rc<RefCell<BvhNode>> {
        if support.is_empty() {
            return Rc::new(RefCell::new(BvhNode::new()));
        }
        let object_count = support.len();
        BvhNode::build_hierarchy(support, 0, object_count - 1)
    }

    pub(super) fn set_serial_index(&mut self, serial_index: usize) {
        self.serial_index = Some(serial_index);
    }

    #[must_use]
    pub(super) fn left(&self) -> &Option<Rc<RefCell<BvhNode>>> {
        &self.left
    }

    #[must_use]
    pub(super) fn right(&self) -> &Option<Rc<RefCell<BvhNode>>> {
        &self.right
    }

    #[must_use]
    pub(super) fn serial_index(&self) -> Option<usize> {
        self.serial_index
    }

    // TODO: rewrite without recursion!

    fn build_hierarchy(support: &mut [Triangle], start: usize, end: usize) -> Rc<RefCell<BvhNode>> {
        assert!(start <= end);
        assert!(support.len() > start);
        assert!(support.len() > end);

        let mut node = BvhNode::new();

        for i in start..=end {
            node.bounding_box = Aabb::merge(node.bounding_box, support[i].bounding_box());
        }

        let axis = node.bounding_box.extent().max_axis();
        let comparator = BvhNode::COMPARATORS[axis as usize];

        let span = end - start;

        if span <= 0 {
            node.content = Some(BvhNodeContent::new(start, end - start + 1));
        } else {
            let mut subarray = support[start..=end].to_vec();
            subarray.sort_by(comparator);

            for (index, object) in subarray.iter().enumerate() {
                support[start + index] = object.clone();
            }

            let middle = start + (span / 2);
            node.left = Some(BvhNode::build_hierarchy(support, start, middle));
            node.right = Some(BvhNode::build_hierarchy(support, middle + 1, end));
            node.axis = axis;

            node.bounding_box = Aabb::merge
            (
                node.left.as_ref().unwrap().borrow().bounding_box,
                node.right.as_ref().unwrap().borrow().bounding_box,
            );
        }

        Rc::new(RefCell::new(node))
    }

    // https://stackoverflow.com/questions/55479683/traversal-of-bounding-volume-hierachy-in-shaders/55483964#55483964
    pub(super) fn populate_links(bvh: &mut BvhNode, next_right_node: Option<Rc<RefCell<BvhNode>>>) {
        if bvh.content.is_none() {
            bvh.hit_node = bvh.left.clone();
            bvh.miss_node = next_right_node.clone();
            bvh.right_offset = bvh.right.clone();

            if let Some(left) = bvh.left.as_mut() {
                BvhNode::populate_links(left.borrow_mut().deref_mut(), bvh.right.clone());
            }
            if let Some(right) = bvh.right.as_mut() {
                BvhNode::populate_links(right.borrow_mut().deref_mut(), next_right_node);
            }
        } else {
            bvh.hit_node = next_right_node.clone();
            bvh.miss_node = next_right_node.clone();
        }
    }

    #[must_use]
    fn box_compare(left: &Triangle, right: &Triangle, axis: Axis) -> Ordering {
        let left_axis_value = left.bounding_box().axis(axis).0;
        let right_axis_value = right.bounding_box().axis(axis).0;

        left_axis_value.partial_cmp(&right_axis_value).unwrap_or(Ordering::Equal)
    }

    #[must_use]
    fn box_x_compare(left: &Triangle, right: &Triangle) -> Ordering {
        BvhNode::box_compare(left, right, Axis::X)
    }

    #[must_use]
    fn box_y_compare(left: &Triangle, right: &Triangle) -> Ordering {
        BvhNode::box_compare(left, right, Axis::Y)
    }

    #[must_use]
    fn box_z_compare(left: &Triangle, right: &Triangle) -> Ordering {
        BvhNode::box_compare(left, right, Axis::Z)
    }

    const COMPARATORS: [fn(&Triangle, &Triangle) -> Ordering; Axis::COUNT] = [
        BvhNode::box_x_compare,
        BvhNode::box_y_compare,
        BvhNode::box_z_compare,
    ];

    pub(crate) const SERIALIZED_QUARTET_COUNT: usize = 3;

    pub(super) fn serialize_by_index_into(&self, container: &mut GpuReadySerializationBuffer) {
        assert!(self.serial_index.is_some(), "index was not set");
        debug_assert!(container.fully_written(), "buffer underflow");

        let index = self.serial_index().unwrap();

        let primitive_kind: f64;
        let primitive_id: f64;
        let primitive_count: f64;
        match self.content.as_ref()
        {
            Some(content) =>
            {
                primitive_kind = 2.0; // TODO: refactor this - 2 is a type for triangle
                primitive_id = content.start_triangle_index() as f64;
                primitive_count = content.triangles_count() as f64;
            }
            None =>
            {
                primitive_kind = Self::GPU_NULL_REFERENCE_MARKER;
                primitive_id = Self::GPU_NULL_REFERENCE_MARKER;
                primitive_count = Self::GPU_NULL_REFERENCE_MARKER;
            },
        }

        container.write_object(index, |writer|{
            writer.write_quartet_f64(
                self.bounding_box.min().x,
                self.bounding_box.min().y,
                self.bounding_box.min().z,
                self.right_offset_index_or_null(),
            );

            writer.write_quartet_f64(
                self.bounding_box.max().x,
                self.bounding_box.max().y,
                self.bounding_box.max().z,
                primitive_kind,
            );

            writer.write_quartet_f64(
                primitive_id,
                primitive_count,
                self.miss_node_index_or_null(),
                self.axis as usize as f64,
            );
        });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::epsilon::DEFAULT_EPSILON;
    use crate::geometry::fundamental_constants::VERTICES_IN_TRIANGLE;
    use crate::geometry::vertex::Vertex;
    use cgmath::{Zero, assert_abs_diff_eq};
    use strum::EnumCount;
    use crate::objects::common_properties::{Linkage, ObjectUid};
    use crate::objects::material_index::MaterialIndex;

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
        assert_abs_diff_eq!(system_under_test.bounding_box, Aabb::new(), epsilon = DEFAULT_EPSILON);
        assert!(system_under_test.content.is_none());
        assert!(system_under_test.hit_node.is_none());
        assert!(system_under_test.miss_node.is_none());
        assert!(system_under_test.right_offset.is_none());
        assert_eq!(system_under_test.axis, Axis::X);
    }

    #[test]
    fn test_single_triangle_support() {
        let triangle = make_triangle([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        let ware = BvhNode::make_for(&mut vec![triangle]);

        let system_under_test = ware.borrow();

        assert!(system_under_test.left.is_none());
        assert!(system_under_test.right.is_none());
        assert_abs_diff_eq!(system_under_test.bounding_box, triangle.bounding_box(), epsilon = DEFAULT_EPSILON);
        assert_eq!(system_under_test.content.as_ref().unwrap().start_triangle_index(), 0);
        assert_eq!(system_under_test.content.as_ref().unwrap().triangles_count(), 1);
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
        let ware = BvhNode::make_for(&mut vec![left, right]);

        let system_under_test = ware.borrow();

        assert_abs_diff_eq!(system_under_test.bounding_box, Aabb::merge(left.bounding_box(), right.bounding_box()), epsilon = DEFAULT_EPSILON);
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
        assert_eq!(BvhNode::index_of_or_null(&Some(Rc::new(RefCell::new(victim)))), expected_index as f64);
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
    fn test_right_offset_index_or_null() {
        let mut node = BvhNode::new();
        assert_eq!(node.right_offset_index_or_null(), BvhNode::GPU_NULL_REFERENCE_MARKER);

        let right_offset_node = Rc::new(RefCell::new(BvhNode::new()));
        let expected_right_offset_index = 5;
        right_offset_node.borrow_mut().set_serial_index(expected_right_offset_index);
        node.right_offset = Some(Rc::clone(&right_offset_node));

        assert_eq!(node.right_offset_index_or_null(), expected_right_offset_index as f64);
    }

    #[test]
    fn test_miss_node_index_or_null() {
        let mut node = BvhNode::new();
        assert_eq!(node.miss_node_index_or_null(), BvhNode::GPU_NULL_REFERENCE_MARKER);

        let miss_node = Rc::new(RefCell::new(BvhNode::new()));
        let expected_miss_node_index = 3;
        miss_node.borrow_mut().set_serial_index(expected_miss_node_index);
        node.miss_node = Some(Rc::clone(&miss_node));

        assert_eq!(node.miss_node_index_or_null(), expected_miss_node_index as f64);
    }

    #[test]
    fn test_make_for_empty_support() {
        let mut support = Vec::new();
        let node = BvhNode::make_for(&mut support);

        assert!(node.borrow().left.is_none());
        assert!(node.borrow().right.is_none());
    }
}
