use crate::geometry::axis::Axis;
use crate::shader::conventions;

pub struct Swizzle {
    rotated_pair: &'static str,
    stable_axis: &'static str,
    final_composition: String,
}

impl Swizzle {
    pub const ROTATED_PAIR_VARIABLE_NAME: &'static str = "rotated";
    
    #[must_use]
    pub fn rotated_pair(&self) -> &'static str {
        self.rotated_pair
    }

    #[must_use]
    pub fn stable_axis(&self) -> &'static str {
        self.stable_axis
    }

    #[must_use]
    pub fn final_composition(&self) -> &str {
        &self.final_composition
    }
}

#[must_use]
pub fn axis_address(axis: Axis) -> &'static str {
    match axis {
        Axis::X => "x",
        Axis::Y => "y",
        Axis::Z => "z",
    }
}

#[must_use]
pub fn morphing_swizzle_from_axis(axis: Axis) -> Swizzle {
    match axis {
        Axis::X => Swizzle {
            rotated_pair: "yz",
            stable_axis: "x",
            final_composition: format!(
                "vec3f({parameter}.x, {rotated})",
                parameter = conventions::PARAMETER_NAME_THE_POINT,
                rotated = Swizzle::ROTATED_PAIR_VARIABLE_NAME,
            ),
        },
        Axis::Y => Swizzle {
            rotated_pair: "xz",
            stable_axis: "y",
            final_composition: format!(
                "vec3f({rotated}.x, {parameter}.y, {rotated}.y)",
                parameter = conventions::PARAMETER_NAME_THE_POINT,
                rotated = Swizzle::ROTATED_PAIR_VARIABLE_NAME,
            ),
        },
        Axis::Z => Swizzle {
            rotated_pair: "xy",
            stable_axis: "z",
            final_composition: format!(
                "vec3f({rotated}, {parameter}.z)",
                parameter = conventions::PARAMETER_NAME_THE_POINT,
                rotated = Swizzle::ROTATED_PAIR_VARIABLE_NAME,
            ),
        },
    }
}