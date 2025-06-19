#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::{Display, Write};
    use float_eq::assert_float_eq;
    use crate::geometry::alias::Point;

    pub(crate) fn assert_all_unique<T: Ord>(victim: &mut Vec<T>) {
        victim.sort();
        if false == victim.windows(2).all(|w| w[0] != w[1]) {
            panic!("no all are unique")
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
    
    pub(crate) fn assert_float_point_equals(left: Point, right: Point, ulps: u64, message_prefix: &str) {
        assert_float_eq!(left.x, right.x, ulps <= ulps, "{}: x component mismatch", message_prefix);
        assert_float_eq!(left.y, right.y, ulps <= ulps, "{}: y component mismatch", message_prefix);
        assert_float_eq!(left.z, right.z, ulps <= ulps, "{}: z component mismatch", message_prefix);
    }
}