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
    pub fn new(target: Rc<dyn Sdf>, axis: Axis, twist_time_scale: f64, twist_amplitude_scale: f64,) -> Rc<Self> {
        assert_gt!(twist_time_scale, 0.0, "twist time scale expected to be positive");
        assert_gt!(twist_amplitude_scale, 0.0, "twist amplitude scale expected to be positive");
        Rc::new(Self { target, axis, twist_time_scale, twist_amplitude_scale, })
    }
}

// struct Swizzle {
//     
// }

impl Sdf for SdfTwisterAlongAxis {
    #[must_use]
    fn produce_body(&self, children_bodies: &mut Stack<ShaderCode<FunctionBody>>, level: Option<usize>) -> ShaderCode<FunctionBody> {
        produce_parameter_transform_body(children_bodies, level, || {
            format!("\
                let whole_object_angle: f32 = {time};\n\
                let whole_object_cos = cos(whole_object_angle);\n\
                let whole_object_sin = sin(whole_object_angle);\n\
                let whole_object_rotor: mat2x2f = mat2x2f(whole_object_cos, whole_object_sin, -whole_object_sin, whole_object_cos);\n\
                let twist_angle: f32 = {position}.x * {twist_amplitude_scale} * sin({time}*{twist_time_scale});\n\
                let twist_cos = cos(twist_angle);\n\
                let twist_sin = sin(twist_angle);\n\
                let twister: mat2x2f = mat2x2f(twist_cos, -twist_sin, twist_sin, twist_cos);\n\
                let rotated: vec2f = (twister * whole_object_rotor) * {position}.yz;\n\
                let {position} = vec3f({position}.x, rotated);",
                time = conventions::PARAMETER_NAME_THE_TIME,
                position = conventions::PARAMETER_NAME_THE_POINT,
                twist_amplitude_scale = format_scalar(self.twist_amplitude_scale),
                twist_time_scale = format_scalar(self.twist_time_scale),
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
        let aabb_size = source_aabb.extent();

        let length = aabb_size[self.axis.as_index()];
        let radius = exclude_axis(aabb_size, self.axis);

        let circumscribed_cylinder = Cylinder::new(source_aabb.center(), self.axis, length, radius.magnitude());

        circumscribed_cylinder.aabb()
    }
}
