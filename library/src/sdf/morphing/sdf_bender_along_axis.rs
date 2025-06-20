use crate::geometry::aabb::Aabb;
use crate::geometry::axis::Axis;
use crate::sdf::framework::n_ary_operations_utils::produce_parameter_transform_body;
use crate::sdf::framework::sdf_base::Sdf;
use crate::sdf::framework::shader_code::{conventions, FunctionBody, ShaderCode};
use crate::sdf::framework::stack::Stack;
use crate::sdf::morphing::utils::circumscribed_cylinder;
use crate::sdf::morphing::morphing_swizzle::{axis_address, morphing_swizzle_from_axis, Swizzle};
use more_asserts::assert_gt;
use std::rc::Rc;
use crate::sdf::framework::shader_formatting_utils::format_scalar;

pub struct SdfBenderAlongAxis {
    target: Rc<dyn Sdf>,
    stable_axis: Axis,
    bend_source_axis: Axis,
    bend_time_scale: f64,
    bend_amplitude_scale: f64,
}

impl SdfBenderAlongAxis {
    #[must_use]
    pub fn new(target: Rc<dyn Sdf>, stable_axis: Axis, bend_source_axis: Axis, bend_time_scale: f64, bend_amplitude_scale: f64) -> Rc<Self> {
        assert_gt!(bend_time_scale, 0.0, "bend time scale expected to be positive");
        assert_gt!(bend_amplitude_scale, 0.0, "bend amplitude scale expected to be positive");
        Rc::new(Self {
            target,
            stable_axis,
            bend_source_axis,
            bend_time_scale,
            bend_amplitude_scale,
        })
    }
}

impl Sdf for SdfBenderAlongAxis {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        let swizzle = morphing_swizzle_from_axis(self.stable_axis);
        produce_parameter_transform_body(children_bodies, level, || {
            format!(
                "\
                let whole_object_angle: f32 = {time};\n\
                let bend_angle: f32 = {position}.{bend_source} * {bend_amplitude_scale} * sin({time}*{bend_time_scale});\n\
                let bend_cos = cos(bend_angle);\n\
                let bend_sin = sin(bend_angle);\n\
                let bender: mat2x2f = mat2x2f(bend_cos, -bend_sin, bend_sin, bend_cos);\n\
                let {rotated}: vec2f = bender * {position}.{rotated_pair};\n\
                let {position} = {composition};",
                time = conventions::PARAMETER_NAME_THE_TIME,
                position = conventions::PARAMETER_NAME_THE_POINT,
                bend_amplitude_scale = format_scalar(self.bend_amplitude_scale),
                bend_time_scale = format_scalar(self.bend_time_scale),
                bend_source = axis_address(self.bend_source_axis),
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
        let circumscribed_cylinder = circumscribed_cylinder(&self.target.aabb(), self.stable_axis);
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
    use rstest::rstest;
    use crate::assert_approx_eq;

    #[rstest]
    #[case(Axis::X, Axis::Y)]
    #[case(Axis::Y, Axis::Z)]
    #[case(Axis::Z, Axis::X)]
    fn test_aabb(#[case] stable_axis: Axis, #[case] bend_source_axis: Axis) {
        let cube_half_size: f64 = 1.0;
        let center = Vector::new(1.0, 3.0, 5.0);
        let shifted_cube = SdfTranslation::new(center, SdfBox::new(Vector::from_value(cube_half_size)));
        let bend_time_scale = 1.0;
        let bend_amplitude_scale = 1.0;
        let system_under_test = SdfBenderAlongAxis::new(shifted_cube, stable_axis, bend_source_axis, bend_time_scale, bend_amplitude_scale);

        let actual_aabb = system_under_test.aabb();
        let actual_extent = actual_aabb.extent();
        let expected_radius = (cube_half_size * cube_half_size + cube_half_size * cube_half_size).sqrt();

        assert_float_point_equals(actual_aabb.center(), Point::from_vec(center), 1, "expected aabb center");
        assert_approx_eq!(f64, actual_extent[stable_axis.as_index()], cube_half_size * 2.0, ulps = 1, "invariant axis extent mismatch");
        assert_approx_eq!(f64, actual_extent[stable_axis.next().as_index()], expected_radius * 2.0, ulps = 1, "bent axis one mismatch");
        assert_approx_eq!(f64, actual_extent[stable_axis.next().next().as_index()], expected_radius * 2.0, ulps = 1, "bent axis two mismatch");
    }

    #[test]
    fn test_code_generation() {
        let bend_time_scale = 1.0;
        let bend_amplitude_scale = 1.0;

        test_unary_operator_body_production(
            |child| SdfBenderAlongAxis::new(child, Axis::Z, Axis::X, bend_time_scale, bend_amplitude_scale),
            "var operand_0: f32;\n{\nlet whole_object_angle: f32 = time;\nlet bend_angle: f32 = point.x * 1.0 * sin(time*1.0);\nlet bend_cos = cos(bend_angle);\nlet bend_sin = sin(bend_angle);\nlet bender: mat2x2f = mat2x2f(bend_cos, -bend_sin, bend_sin, bend_cos);\nlet rotated: vec2f = bender * point.xy;\nlet point = vec3f(rotated, point.z);\n{\noperand_0 = ?_left;\n}\n}\nreturn operand_0;",
        );
    }
}
