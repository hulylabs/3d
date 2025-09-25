use derive_more::Display;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Display, Hash)]
pub struct ProceduralTextureUid(pub usize);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedural_texture_index_creation() {
        let expected_value = 17;
        let system_under_test = ProceduralTextureUid(expected_value);
        assert_eq!(system_under_test.0, expected_value);
    }

    #[test]
    fn test_procedural_texture_index_copy() {
        let system_under_test = ProceduralTextureUid(10);
        let copied_value = system_under_test;
        assert_eq!(system_under_test, copied_value);
    }

    #[test]
    fn test_procedural_texture_index_clone() {
        let system_under_test = ProceduralTextureUid(25);
        let cloned_value = system_under_test.clone();
        assert_eq!(system_under_test, cloned_value);
    }

    #[test]
    fn test_procedural_texture_index_debug() {
        let system_under_test = ProceduralTextureUid(100);
        let debug_view = format!("{:?}", system_under_test);
        assert_eq!(debug_view, "ProceduralTextureUid(100)");
    }

    #[test]
    fn test_procedural_texture_index_display() {
        let expected_value = 123;
        let system_under_test = ProceduralTextureUid(expected_value);
        let string_view = format!("{}", system_under_test);
        assert_eq!(string_view, expected_value.to_string());
    }

    #[test]
    fn test_procedural_texture_index_equality() {
        let system_under_test = ProceduralTextureUid(50);
        let equal_value = ProceduralTextureUid(system_under_test.0);
        let different_value = ProceduralTextureUid(system_under_test.0 + 1);

        assert_eq!(system_under_test, equal_value);
        assert_ne!(system_under_test, different_value);
        assert_ne!(equal_value, different_value);
    }
}