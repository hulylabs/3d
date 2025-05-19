#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::Write;
    
    pub(crate) fn assert_eq(left: &[f32], right: &[f32], epsilon: f32) {
        assert_eq!(left.len(), right.len(), "ranges have different lengths");

        let mut buffer = String::new();
        for (i, (x, y)) in left.iter().zip(right.iter()).enumerate() {
            if (x - y).abs() > epsilon {
                write!(&mut buffer, "values at index {} differ: {} vs {}\n", i, x, y, ).unwrap();   
            }
        }
        
        if !buffer.is_empty() {
            panic!("{}", buffer);
        }
    }
}