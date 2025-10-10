#language slang 2026

module glyph_sdf;

import glyphs_storage;

// Ahtung! This constant is duplicated in the Rust code for the sake of AABB evaluation.
static const float Z_EXTRUSION_HALF_THICKNESS = 0.01;

public func median(r: float, g: float, b: float) -> float{
    return max(min(r, g), min(max(r, g), b));
}

public func sdf_glyph<Configuration: IGlyphsStorage>(glyph_index: uint32_t, position: float3) -> float {
    let in_plane_distance = sdf_glyph_xy<Configuration>(glyph_index, position.xy);
    let distance_to_extruded = extrude(position, in_plane_distance, Z_EXTRUSION_HALF_THICKNESS);
    return distance_to_extruded;
}

public func sdf_glyph_xy<Configuration : IGlyphsStorage>(glyph_index: uint32_t, position: float2)->float {
    let glyph_texture_location = Configuration.locate_glyph(glyph_index);
    let msdf = Configuration.sample(glyph_texture_location.evaluate_uv(position));
    let texture_signed_distance = (1.0 - median(msdf.x, msdf.y, msdf.z)) - 0.5; // [inside .. outside] = [1 .. 0] -> [-0.5 .. +0.5]
    let world_signed_distance = Configuration.texture_to_world_scale() * texture_signed_distance;
    return world_signed_distance;
}

func extrude(position: float3, plane_sdf: float, depth: float) -> float {
    let w = float2(plane_sdf, abs(position.z) - depth);
    return min(max(w.x, w.y), 0.0) + length(max(w, 0.0));
}
