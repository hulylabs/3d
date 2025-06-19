use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use crate::sdf::framework::n_ary_operations_utils::produce_parameter_transform_body;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::framework::shader_formatting_utils::format_scalar;
use crate::sdf::framework::stack::Stack;
use crate::sdf::morphing::utils::circumscribed_cylinder;
use crate::sdf::morphing::morphing_swizzle::{morphing_swizzle_from_axis, Swizzle};
use more_asserts::assert_gt;
use std::rc::Rc;

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
        Rc::new(Self {
            target,
            axis,
            twist_time_scale,
            twist_amplitude_scale,
        })
    }
}

impl Sdf for SdfTwisterAlongAxis {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        let swizzle = morphing_swizzle_from_axis(self.axis);
        produce_parameter_transform_body(children_bodies, level, || {
            format!(
                "\
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
                stable_axis = swizzle.stable_axis(),
                rotated_pair = swizzle.rotated_pair(),
                composition = swizzle.final_composition(),
                rotated = Swizzle::ROTATED_PAIR_VARIABLE_NAME,
            )
        })
    }

    #[must_use]
    fn descendants(&self) -> Vec<Rc<dyn Sdf>> {
        vec![self.target.clone()]
    }

    #[must_use]
    fn aabb(&self) -> Aabb {
        let circumscribed_cylinder = circumscribed_cylinder(&self.target.aabb(), self.axis);
        circumscribed_cylinder.aabb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::alias::{Point, Vector};
    use crate::sdf::framework::n_ary_operations_utils::tests::test_unary_operator_body_production;
    use crate::sdf::object::sdf_box::SdfBox;
    use crate::sdf::transformation::sdf_translation::SdfTranslation;
    use crate::utils::tests::assert_utils::tests::assert_float_point_equals;
    use cgmath::{Array, EuclideanSpace};
    use float_eq::assert_float_eq;
    use rstest::rstest;

    #[rstest]
    #[case(Axis::X)]
    #[case(Axis::Y)]
    #[case(Axis::Z)]
    fn test_aabb(#[case] axis: Axis) {
        let cube_half_size: f64 = 1.0;
        let center = Vector::new(1.0, 3.0, 5.0);
        let shifted_cube = SdfTranslation::new(center, SdfBox::new(Vector::from_value(cube_half_size)));
        let twist_time_scale = 1.0;
        let twist_amplitude_scale = 1.0;
        let system_under_test = SdfTwisterAlongAxis::new(shifted_cube, axis, twist_time_scale, twist_amplitude_scale);

        let actual_aabb = system_under_test.aabb();
        let actual_extent = actual_aabb.extent();
        let expected_radius = (cube_half_size * cube_half_size + cube_half_size * cube_half_size).sqrt();

        assert_float_point_equals(actual_aabb.center(), Point::from_vec(center), 1, "expected aabb center");
        assert_float_eq!(actual_extent[axis.as_index()], cube_half_size * 2.0, ulps <= 1, "invariant axis extent mismatch");
        assert_float_eq!(actual_extent[axis.next().as_index()], expected_radius * 2.0, ulps <= 1, "twisted axis one mismatch");
        assert_float_eq!(actual_extent[axis.next().next().as_index()], expected_radius * 2.0, ulps <= 1, "twisted axis two mismatch");
    }

    #[test]
    fn test_code_generation() {
        let twist_time_scale = 1.0;
        let twist_amplitude_scale = 1.0;

        test_unary_operator_body_production(
            |child| SdfTwisterAlongAxis::new(child, Axis::Z, twist_time_scale, twist_amplitude_scale),
            "var operand_0: f32;\n{\nlet whole_object_angle: f32 = time;\nlet whole_object_cos = cos(whole_object_angle);\nlet whole_object_sin = sin(whole_object_angle);\nlet whole_object_rotor: mat2x2f = mat2x2f(whole_object_cos, whole_object_sin, -whole_object_sin, whole_object_cos);\nlet twist_angle: f32 = point.z * 1.0 * sin(time*1.0);\nlet twist_cos = cos(twist_angle);\nlet twist_sin = sin(twist_angle);\nlet twister: mat2x2f = mat2x2f(twist_cos, -twist_sin, twist_sin, twist_cos);\nlet rotated: vec2f = (twister * whole_object_rotor) * point.xy;\nlet point = vec3f(rotated, point.z);\n{\noperand_0 = ?_left;\n}\n}\nreturn operand_0;",
        );
    }
}
