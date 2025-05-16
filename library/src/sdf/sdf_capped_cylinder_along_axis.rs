use crate::geometry::alias::Point;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode};
use crate::sdf::shader_formatting_utils::{format_scalar, format_sdf_parameter};
use crate::sdf::stack::Stack;
use cgmath::EuclideanSpace;
use std::rc::Rc;
use crate::geometry::axis::Axis;

pub struct SdfCappedCylinderAlongAxis {
    half_height: f64,
    radius: f64,
    center: Point,
    axis: Axis,
}

impl SdfCappedCylinderAlongAxis {
    #[must_use]
    pub fn new_offset(axis: Axis, half_height: f64, radius: f64, center: Point) -> Rc<Self> {
        assert!(half_height > 0.0, "height must be > 0");
        assert!(radius > 0.0, "radius must be > 0");
        Rc::new(Self { half_height, radius, center, axis })
    }

    #[must_use]
    pub fn new(axis: Axis, half_height: f64, radius: f64) -> Rc<Self> {
        assert!(half_height > 0.0, "height must be > 0");
        assert!(radius > 0.0, "radius must be > 0");
        Self::new_offset(axis, half_height, radius, Point::origin())
    }
}

struct Swizzling {
    cylinder_axis: &'static str,
    radius_axes: &'static str,
}

#[must_use]
fn swizzle(axis: Axis) -> Swizzling {
    match axis {
        Axis::X => {Swizzling { cylinder_axis: "x", radius_axes: "yz" }}
        Axis::Y => {Swizzling { cylinder_axis: "y", radius_axes: "xz" }}
        Axis::Z => {Swizzling { cylinder_axis: "z", radius_axes: "xy" }}
    }
}

impl Sdf for SdfCappedCylinderAlongAxis {
    #[must_use]
    fn produce_body(&self, _children_bodies: &mut Stack<ShaderCode<FunctionBody>>, _level: Option<usize>) -> ShaderCode<FunctionBody> {
        let swizzling = swizzle(self.axis);
        ShaderCode::<FunctionBody>::new(format!(
            "let d = abs(vec2f(length({parameter}.{radius_axes}), {parameter}.{cylinder_axis})) - vec2f({radius}, {half_height});\n\
             return min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));",
            parameter = format_sdf_parameter(self.center),
            radius_axes = swizzling.radius_axes,
            cylinder_axis = swizzling.cylinder_axis,
            radius = format_scalar(self.radius),
            half_height = format_scalar(self.half_height),
        ))
    }

    #[must_use]
    fn children(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use super::*;

    #[test]
    fn test_children() {
        let system_under_test = SdfCappedCylinderAlongAxis::new(Axis::Y, 2.0, 1.0);
        assert!(system_under_test.children().is_empty())
    }

    #[rstest]
    #[case(Axis::X,  "let d = abs(vec2f(length(point.yz), point.x)) - vec2f(5.0, 19.0);\nreturn min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));")]
    #[case(Axis::Y,  "let d = abs(vec2f(length(point.xz), point.y)) - vec2f(5.0, 19.0);\nreturn min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));")]
    #[case(Axis::Z,  "let d = abs(vec2f(length(point.xy), point.z)) - vec2f(5.0, 19.0);\nreturn min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));")]
    fn test_float_formatting_integer_value(#[case] axis: Axis, #[case] expected_body: &str) {
        let height = 19.0;
        let radius = 5.0;
        let system_under_test = SdfCappedCylinderAlongAxis::new(axis, height, radius);

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));
        
        assert_eq!(actual_body.as_str(), expected_body);
    }

    #[rstest]
    #[case(Axis::X,  "let d = abs(vec2f(length((point-vec3f(1.0,2.0,3.0)).yz), (point-vec3f(1.0,2.0,3.0)).x)) - vec2f(5.0, 19.0);\nreturn min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));")]
    #[case(Axis::Y,  "let d = abs(vec2f(length((point-vec3f(1.0,2.0,3.0)).xz), (point-vec3f(1.0,2.0,3.0)).y)) - vec2f(5.0, 19.0);\nreturn min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));")]
    #[case(Axis::Z,  "let d = abs(vec2f(length((point-vec3f(1.0,2.0,3.0)).xy), (point-vec3f(1.0,2.0,3.0)).z)) - vec2f(5.0, 19.0);\nreturn min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));")]
    fn test_offset_construction(#[case] axis: Axis, #[case] expected_body: &str) {
        let height = 19.0;
        let radius = 5.0;
        let system_under_test = SdfCappedCylinderAlongAxis::new_offset(axis, height, radius, Point::new(1.0, 2.0, 3.0));

        let actual_body = system_under_test.produce_body(&mut Stack::new(), Some(0));

        assert_eq!(actual_body.as_str(), expected_body);
    }
}