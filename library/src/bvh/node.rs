use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use crate::objects::triangle::Triangle;
use crate::serialization::helpers::{floats_count, GpuFloatBufferFiller};
use crate::serialization::serializable_for_gpu::SerializableForGpu;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::DerefMut;
use std::rc::Rc;

pub(super) struct BvhNode {
    left: Option<Rc<RefCell<BvhNode>>>,
    right: Option<Rc<RefCell<BvhNode>>>,
    bounding_box: Aabb,
    content: Option<Triangle>,
    start_index: Option<usize>,
    triangles_count: usize,
    serial_index: Option<usize>,
    hit_node: Option<Rc<RefCell<BvhNode>>>,
    miss_node: Option<Rc<RefCell<BvhNode>>>,
    right_offset: Option<Rc<RefCell<BvhNode>>>,
    axis: Axis,
}

// TODO: rewrite without recursion!

impl BvhNode {

    #[must_use]
    fn new() -> Self {
        Self {
            left: None,
            right: None,
            bounding_box: Aabb::new(),
            content: None,
            start_index: None,
            triangles_count: 0,
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
    fn start_index_or_null(&self) -> f64 {
        match self.start_index {
            None => BvhNode::GPU_NULL_REFERENCE_MARKER,
            Some(index) => index as f64,
        }
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

    // TODO: triangles copying and aabb evaluations may be slow?

    fn build_hierarchy(support: &mut[Triangle], start: usize, end: usize) -> Rc<RefCell<BvhNode>> {
        assert!(start <= end);
        assert!(support.len() > start);
        assert!(support.len() > end);

        let mut node = BvhNode::new();

        for i in start..=end {
            node.bounding_box = Aabb::merge(node.bounding_box, support[i].bounding_box());
        }

        let extent = node.bounding_box.extent();
        let mut axis = Axis::X;
        if extent[1] > extent[0] {
            axis = Axis::Y;
        }
        if extent[2] > extent[axis as usize] {
            axis = Axis::Z;
        }

        let comparator = match axis {
            Axis::X => BvhNode::box_x_compare,
            Axis::Y => BvhNode::box_y_compare,
            Axis::Z => BvhNode::box_z_compare,
        };

        let span = end - start;

        if span <= 0 {
            node.content = Some(support[start].clone());
            node.start_index = Some(start);
            node.triangles_count = end - start + 1;
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

            node.bounding_box = Aabb::merge(node.left.as_ref().unwrap().borrow().bounding_box, node.right.as_ref().unwrap().borrow().bounding_box);
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
    fn box_compare(a: &Triangle, b: &Triangle, axis: Axis) -> Ordering {
        let a_axis_value = a.bounding_box().axis(axis).0;
        let b_axis_value = b.bounding_box().axis(axis).0;

        a_axis_value.partial_cmp(&b_axis_value).unwrap_or(Ordering::Equal)
    }

    #[must_use]
    fn box_x_compare(a: &Triangle, b: &Triangle) -> Ordering {
        BvhNode::box_compare(a, b, Axis::X)
    }

    #[must_use]
    fn box_y_compare(a: &Triangle, b: &Triangle) -> Ordering {
        BvhNode::box_compare(a, b, Axis::Y)
    }

    #[must_use]
    fn box_z_compare(a: &Triangle, b: &Triangle) -> Ordering {
        BvhNode::box_compare(a, b, Axis::Z)
    }

    const SERIALIZED_QUARTET_COUNT: usize = 3;
}

// TODO: is it more efficient to use 'Surface Area Heuristic'?

impl SerializableForGpu for BvhNode {
    const SERIALIZED_SIZE_FLOATS: usize = floats_count(BvhNode::SERIALIZED_QUARTET_COUNT);

    fn serialize_into(&self, container: &mut [f32]) {
        assert!(container.len() >= BvhNode::SERIALIZED_SIZE_FLOATS, "buffer size is too small");

        let mut index = 0;

        container.write_and_move_next(self.bounding_box.min().x, &mut index);
        container.write_and_move_next(self.bounding_box.min().y, &mut index);
        container.write_and_move_next(self.bounding_box.min().z, &mut index);
        container.write_and_move_next(self.right_offset_index_or_null(), &mut index);

        container.write_and_move_next(self.bounding_box.max().x, &mut index);
        container.write_and_move_next(self.bounding_box.max().y, &mut index);
        container.write_and_move_next(self.bounding_box.max().z, &mut index);

        if self.content.is_some() {
            container.write_and_move_next(2.0, &mut index); // TODO: refactor this - 2 is a type for triangle
            container.write_and_move_next(self.start_index_or_null(), &mut index);
            container.write_and_move_next(self.triangles_count as f64, &mut index);
        } else {
            container.write_and_move_next(Self::GPU_NULL_REFERENCE_MARKER, &mut index);
            container.write_and_move_next(Self::GPU_NULL_REFERENCE_MARKER, &mut index);
            container.write_and_move_next(Self::GPU_NULL_REFERENCE_MARKER, &mut index);
        }

        container.write_and_move_next(self.miss_node_index_or_null(), &mut index);
        container.write_and_move_next(self.axis as usize as f64, &mut index);

        assert_eq!(index, BvhNode::SERIALIZED_SIZE_FLOATS);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::epsilon::DEFAULT_EPSILON;
    use crate::geometry::fundamental_constants::VERTICES_IN_TRIANGLE;
    use crate::geometry::vertex::Vertex;
    use crate::objects::triangle::{MeshIndex, TriangleIndex};
    use cgmath::{assert_abs_diff_eq, Zero};
    use strum::EnumCount;

    #[test]
    fn test_empty_support() {
        let ware = BvhNode::make_for(&mut vec![]);

        let system_under_test = ware.borrow();
        let epsilon = DEFAULT_EPSILON;

        assert!(system_under_test.left.is_none());
        assert!(system_under_test.right.is_none());
        assert_abs_diff_eq!(system_under_test.bounding_box, Aabb::new(), epsilon=epsilon);
        assert!(system_under_test.content.is_none());
        assert!(system_under_test.start_index.is_none());
        assert_eq!(system_under_test.triangles_count, 0);
        assert!(system_under_test.hit_node.is_none());
        assert!(system_under_test.miss_node.is_none());
        assert!(system_under_test.right_offset.is_none());
        assert_eq!(system_under_test.axis, Axis::X);
    }

    pub (crate) fn make_triangle(vertex_data: [f64; VERTICES_IN_TRIANGLE * Axis::COUNT]) -> Triangle {
        Triangle::new(
            Vertex::new(Point::new(vertex_data[0], vertex_data[1], vertex_data[2], ), Vector::zero()),
            Vertex::new(Point::new(vertex_data[3], vertex_data[4], vertex_data[5], ), Vector::zero()),
            Vertex::new(Point::new(vertex_data[6], vertex_data[7], vertex_data[8], ), Vector::zero()),
            TriangleIndex(0),
            MeshIndex(0),
        )
    }

    #[test]
    fn test_single_triangle_support() {
        let triangle = make_triangle([
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ]);
        let ware = BvhNode::make_for(&mut vec![triangle]);

        let system_under_test = ware.borrow();
        let epsilon = DEFAULT_EPSILON;

        assert!(system_under_test.left.is_none());
        assert!(system_under_test.right.is_none());
        assert_abs_diff_eq!(system_under_test.bounding_box, triangle.bounding_box(), epsilon=epsilon);
        assert_abs_diff_eq!(system_under_test.content.unwrap(), triangle);
        assert_eq!(system_under_test.start_index.unwrap(), 0);
        assert_eq!(system_under_test.triangles_count, 1);
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
        let epsilon = DEFAULT_EPSILON;

        assert_abs_diff_eq!(system_under_test.bounding_box, Aabb::merge(left.bounding_box(), right.bounding_box()), epsilon=epsilon);
        assert!(system_under_test.content.is_none());
        assert!(system_under_test.start_index.is_none());
        assert_eq!(system_under_test.triangles_count, 0);
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
}