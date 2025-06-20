use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Point;
use crate::geometry::axis::Axis;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::framework::shader_formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use std::rc::Rc;

pub struct SdfCappedCylinderAlongAxis {
    half_height: f64,
    radius: f64,
    axis: Axis,
}

impl SdfCappedCylinderAlongAxis {
    #[must_use]
    pub fn new(axis: Axis, half_height: f64, radius: f64) -> Rc<Self> {
        assert!(half_height > 0.0, "height must be > 0");
        assert!(radius > 0.0, "radius must be > 0");
        Rc::new(Self { half_height, radius, axis })
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
            parameter = conventions::PARAMETER_NAME_THE_POINT,
            radius_axes = swizzling.radius_axes,
            cylinder_axis = swizzling.cylinder_axis,
            radius = format_scalar(self.radius),
            half_height = format_scalar(self.half_height),
        ))
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        Vec::new()
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let mut min = Point::new(0.0, 0.0, 0.0);
        let mut max = Point::new(0.0, 0.0, 0.0);

        min[self.axis.as_index()] = -self.half_height;
        max[self.axis.as_index()] =  self.half_height;

        min[self.axis.next().as_index()] = -self.radius;
        max[self.axis.next().as_index()] =  self.radius;

        min[self.axis.next().next().as_index()] = -self.radius;
        max[self.axis.next().next().as_index()] =  self.radius;

        Aabb::from_points(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_children() {
        let system_under_test = SdfCappedCylinderAlongAxis::new(Axis::Y, 2.0, 1.0);
        assert!(system_under_test.descendants().is_empty())
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
}