use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use crate::geometry::cylinder::Cylinder;
use crate::geometry::utils::exclude_axis;
use crate::sdf::n_ary_operations_utils::produce_parameter_transform_body;
use crate::sdf::sdf_base::Sdf;
use crate::sdf::shader_code::{FunctionBody, ShaderCode, conventions};
use crate::sdf::stack::Stack;
use cgmath::InnerSpace;
use std::rc::Rc;
use more_asserts::assert_gt;
use crate::sdf::shader_formatting_utils::format_scalar;

pub struct SdfTwisterAlongAxis {
    target: Rc<dyn Sdf>,
    axis: Axis,
    twist_time_scale: f64,
    twist_amplitude_scale: f64,
}

impl SdfTwisterAlongAxis {
    #[must_use]
    pub fn new(target: Rc<dyn Sdf>, axis: Axis, twist_time_scale: f64, twist_amplitude_scale: f64) -> Rc<Self> {
        assert_gt!(twist_time_scale, 0.0, "twist time scale expected to be positive");
        assert_gt!(twist_amplitude_scale, 0.0, "twist amplitude scale expected to be positive");
        Rc::new(Self { target, axis, twist_time_scale, twist_amplitude_scale })
    }
}

struct Swizzle {
    rotated_pair: &'static str,
    stable_axis: &'static str,
    final_composition: String,
}

const ROTATED_PAIR_NAME: &str = "rotated";

#[must_use]
fn swizzle_from_axis(axis: Axis) -> Swizzle {
    match axis {
        Axis::X => {
            Swizzle {
                rotated_pair: "yz",
                stable_axis: "x",
                final_composition: format!("vec3f({parameter}.x, {rotated})",
                    parameter=conventions::PARAMETER_NAME_THE_POINT,
                    rotated=ROTATED_PAIR_NAME),
            }
        }
        Axis::Y => {
            Swizzle {
                rotated_pair: "xz",
                stable_axis: "y",
                final_composition: format!("vec3f({rotated}.x, {parameter}.y, {rotated}.z)",
                    parameter=conventions::PARAMETER_NAME_THE_POINT,
                    rotated=ROTATED_PAIR_NAME),
            }
        }
        Axis::Z => {
            Swizzle {
                rotated_pair: "xy",
                stable_axis: "z",
                final_composition: format!("vec3f({rotated}, {parameter}.z)",
                    parameter=conventions::PARAMETER_NAME_THE_POINT,
                    rotated=ROTATED_PAIR_NAME),
            }
        }
    }
}

impl Sdf for SdfTwisterAlongAxis {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        let swizzle = swizzle_from_axis(self.axis);
        produce_parameter_transform_body(children_bodies, level, || {
            format!("\
                let whole_object_angle: f32 = {time};\n\
                let whole_object_cos = cos(whole_object_angle);\n\
                let whole_object_sin = sin(whole_object_angle);\n\
                let whole_object_rotor: mat2x2f = mat2x2f(whole_object_cos, whole_object_sin, -whole_object_sin, whole_object_cos);\n\
                let twist_angle: f32 = {position}.{stable_axis} * {twist_amplitude_scale} * sin({time}*{twist_time_scale});\n\
                let twist_cos = cos(twist_angle);\n\
                let twist_sin = sin(twist_angle);\n\
                let twister: mat2x2f = mat2x2f(twist_cos, -twist_sin, twist_sin, twist_cos);\n\
                let {rotated}: vec2f = (twister * whole_object_rotor) * {position}.{rotated_pair};\n\
                let {position} = {composition};",
                    time = conventions::PARAMETER_NAME_THE_TIME,
                    position = conventions::PARAMETER_NAME_THE_POINT,
                    twist_amplitude_scale = format_scalar(self.twist_amplitude_scale),
                    twist_time_scale = format_scalar(self.twist_time_scale),
                    stable_axis = swizzle.stable_axis,
                    rotated_pair = swizzle.rotated_pair,
                    composition = swizzle.final_composition,
                    rotated=ROTATED_PAIR_NAME,
            )
        })
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.target.clone()]
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let source_aabb = self.target.aabb();
        let source_aabb_extent = source_aabb.extent();

        let length = source_aabb_extent[self.axis.as_index()];
        let radius = exclude_axis(source_aabb_extent, self.axis) / 2.0;

        let circumscribed_cylinder = Cylinder::new(source_aabb.center(), self.axis, length, radius.magnitude());

        circumscribed_cylinder.aabb()
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::epsilon::DEFAULT_EPSILON_F64;
use cgmath::{assert_abs_diff_eq, Array, EuclideanSpace};
    use crate::geometry::alias::{Point, Vector};
    use crate::sdf::sdf_box::SdfBox;
    use crate::sdf::sdf_translation::SdfTranslation;
    use super::*;

    // #[rstest]
    // #[case(Axis::X)]
    // #[case(Axis::Y)]
    // #[case(Axis::Z)]
    // fn test_aabb(#[case] axis: Axis) {
    #[test]
    fn test_aabb() {
        let axis: Axis = Axis::X;
        let cube_half_size: f64 = 1.0;
        let center = Vector::new(1.0, 3.0, 5.0);
        let shifted_cube = SdfTranslation::new(center, SdfBox::new(Vector::from_value(cube_half_size)));
        let twist_time_scale = 1.0;
        let twist_amplitude_scale = 1.0;
        let system_under_test = SdfTwisterAlongAxis::new(shifted_cube, axis, twist_time_scale, twist_amplitude_scale);

        let actual_aabb = system_under_test.aabb();
        let actual_extent = actual_aabb.extent();
        let expected_radius = (cube_half_size * cube_half_size + cube_half_size * cube_half_size).sqrt();

        assert_eq!(actual_aabb.center(), Point::from_vec(center));
        assert_abs_diff_eq!(actual_extent[axis.as_index()], cube_half_size * 2.0, epsilon = DEFAULT_EPSILON_F64);//, "invariant axis extent mismatch");
        assert_abs_diff_eq!(actual_extent[axis.next().as_index()], expected_radius * 2.0, epsilon = DEFAULT_EPSILON_F64);//, "twisted axis one mismatch");
        assert_abs_diff_eq!(actual_extent[axis.next().next().as_index()], expected_radius * 2.0, epsilon = DEFAULT_EPSILON_F64);//, "twisted axis two mismatch");
    }

    #[test]
    fn test_code_generation() {
        //let shifted_cube = SdfTranslation::new(center, SdfBox::new(Vector::from_value(cube_half_size)));
    }
}