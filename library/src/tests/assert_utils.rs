#[cfg(test)]
pub(crate) mod tests {
    pub(crate) fn assert_eq(left: &[f32], right: &[f32], epsilon: f32) {
        assert_eq!(left.len(), right.len(), "ranges have different lengths");

        for (i, (x, y)) in left.iter().zip(right.iter()).enumerate() {
            assert!(
                (x - y).abs() < epsilon,
                "Values at index {} differ: {} vs {}",
                i, x, y
            );
        }
    }
}