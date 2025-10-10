pub const RESERVED_SDF_NAME_PREFIX: &str = "_huly_sdf_";

#[must_use]
pub(crate) fn make_reserved_name(suffix: &str) -> String {
    format!("{}{}", RESERVED_SDF_NAME_PREFIX, suffix)
}