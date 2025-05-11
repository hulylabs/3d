use std::fmt::{Display, Formatter};
use float_pretty_print::PrettyPrintFloat;
use crate::geometry::alias::{Point, Vector};

pub(super) struct ShaderReadyFloat {
    value: f32,
}

impl ShaderReadyFloat {
    const FLOAT_DECIMAL_DIGITS_TO_KEEP: u32 = 13;
    const SCALER: f64 = 10usize.pow(ShaderReadyFloat::FLOAT_DECIMAL_DIGITS_TO_KEEP) as f64;
    
    #[must_use]
    pub(super) const fn new(value: f64) -> Self {
        let scaled = (value * ShaderReadyFloat::SCALER) as i64;
        let truncated = (scaled as f64 / ShaderReadyFloat::SCALER) as f32;
        Self { value: truncated }
    }

    #[must_use]
    pub(super) fn format(&self) -> String {
        PrettyPrintFloat(self.value as f64).to_string()
    }
}

impl Display for ShaderReadyFloat {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.format().as_str())
    }
}

#[must_use]
pub(super) fn format_vector(target: Vector) -> String {
    format_three_dee_vector(ShaderReadyFloat::new(target.x), ShaderReadyFloat::new(target.y), ShaderReadyFloat::new(target.z), )
}

#[must_use]
pub(super) fn format_point(target: Point) -> String {
    format_three_dee_vector(ShaderReadyFloat::new(target.x), ShaderReadyFloat::new(target.y), ShaderReadyFloat::new(target.z), )
}

#[must_use]
fn format_three_dee_vector(x: ShaderReadyFloat, y: ShaderReadyFloat, z: ShaderReadyFloat) -> String {
    format!("vec3f({},{},{})", x.format(), y.format(), z.format())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case( 0.0,  "0.0")]
    #[case( 1.0,  "1.0")]
    #[case(-7.0, "-7.0")]
    fn test_float_formatting_integer_value(#[case] value: f64, #[case] expected: &str) {
        let system_under_test = ShaderReadyFloat::new(value);
        assert_eq!(system_under_test.format(), expected);
    }

    #[rstest]
    #[case( 0.1 ,  "0.1000000015")]
    #[case( 0.37,  "0.3700000048")]
    #[case(-0.1 , "-0.100000001")]
    #[case( 1e-7,  "1e-7")]
    #[case(-1e-7, "-1e-7")]
    fn test_float_formatting_less_by_abs_than_one(#[case] value: f64, #[case] expected: &str) {
        let system_under_test = ShaderReadyFloat::new(value);
        assert_eq!(system_under_test.format(), expected);
    }
    
    #[test]
    fn test_vector_formatting() {
        let actual_format = format_vector(Vector::new(1.0, 2.0, 3.0));
        let expected_format = "vec3f(1.0,2.0,3.0)";
        assert_eq!(expected_format, actual_format);
    }

    #[test]
    fn test_point_formatting() {
        let actual_format = format_point(Point::new(3.0, 5.0, 7.0));
        let expected_format = "vec3f(3.0,5.0,7.0)";
        assert_eq!(expected_format, actual_format);
    }

    #[test]
    fn test_truncation() {
        let system_under_test = ShaderReadyFloat::new(7.00000000000003);
        assert_eq!(system_under_test.format(), "7.0");
    }
}