// This data is duplicated in shader code in the glyph_sdf.slang file.

const GLYPH_LOCAL_Z_EXTRUSION_HALF_THICKNESS: f64 = 0.01;
pub(super) const GLYPH_LOCAL_Z_EXTRUSION_THICKNESS: f64 = GLYPH_LOCAL_Z_EXTRUSION_HALF_THICKNESS * 2.0;

#[cfg(test)]
pub(crate) mod tests {

    pub(crate) const ASCII_PRINTABLE_GLYPHS_COUNT: usize = 95;

}