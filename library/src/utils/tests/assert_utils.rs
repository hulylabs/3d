#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::{Display, Write};
    use crate::geometry::alias::Point;

    pub(crate) fn assert_all_unique<T: Ord>(victim: &mut Vec<T>) {
        victim.sort();
        if false == victim.windows(2).all(|w| w[0] != w[1]) {
            panic!("not all elements of the vector are unique")
        }
    }
    
    pub(crate) fn assert_eq(left: &[f32], right: &[f32], epsilon: f32) {
        assert_eq!(left.len(), right.len(), "ranges have different lengths");

        let mut buffer = String::new();
        for (i, (x, y)) in left.iter().zip(right.iter()).enumerate() {
            if (x - y).abs() > epsilon {
                write!(&mut buffer, "values at index {} differ: left = {} vs right = {}\n", i, x, y, ).unwrap();   
            }
        }
        
        if !buffer.is_empty() {
            panic!("{}", buffer);
        }
    }

    pub(crate) fn assert_all_not_equal<T: PartialEq>(left: &[T], right: &[T]) {
        assert_eq!(left.len(), right.len(), "ranges have different lengths");

        let mut buffer = String::new();
        for (i, (x, y)) in left.iter().zip(right.iter()).enumerate() {
            if x == y {
                write!(&mut buffer, "values at index {} are equal\n", i,).unwrap();
            }
        }

        if !buffer.is_empty() {
            panic!("{}", buffer);
        }
    }

    pub(crate) fn assert_all_items_equal<T: PartialEq + Display>(target: &[T], reference: T) {
        for i in 0..target.len() {
            if target[i] != reference {
                panic!("element '{element}' at index {index} differs from reference", element = target[i], index = i);
            }
        }
    }

    #[macro_export]
    macro_rules! assert_approx_eq {
        ($typ:ty, $left:expr, $right:expr, ulps = $ulps:expr $(, $($arg:tt)+)?) => {{
            let left_val = $left;
            let right_val = $right;
            if !float_cmp::approx_eq!($typ, left_val, right_val, ulps = $ulps) {
                panic!(
                    "assertion failed: `(left approx_eq right)`\n  left: `{:?}`,\n right: `{:?}`,\n{}",
                    left_val,
                    right_val,
                    format_args!($($($arg)+)?)
                );
            }
        }};
    }
    
    pub(crate) fn assert_float_point_equals(left: Point, right: Point, ulps: i64, message_prefix: &str) {
        assert_approx_eq!(f64, left.x, right.x, ulps = ulps, "{}: x component mismatch", message_prefix);
        assert_approx_eq!(f64, left.y, right.y, ulps = ulps, "{}: y component mismatch", message_prefix);
        assert_approx_eq!(f64, left.z, right.z, ulps = ulps, "{}: z component mismatch", message_prefix);
    }

    #[test]
    fn test_assert_approx_eq() {
        let a: f32 = 0.15 + 0.15 + 0.15;
        let b: f32 = 0.1 + 0.1 + 0.25;
        assert_approx_eq!(f32, a, b, ulps = 1, "test message");
    }

    #[test]
    #[should_panic]
    fn test_assert_approx_eq_panic() {
        let a: f32 = 0.00000000005;
        let b: f32 = 0.00000000001;
        assert_approx_eq!(f32, a, b, ulps = 1, "test message");
    }
}