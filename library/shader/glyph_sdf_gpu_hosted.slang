#language slang 2026

module glyph_sdf_pgu_hosted;

import glyphs_storage;
import glyph_sdf;
import uniforms;

[vk::binding(3, 0)]
SamplerState glyphs_atlas_sampler;
[vk::binding(4, 0)]
Texture2D<float4> glyphs_atlas;
[vk::binding(5, 0)]
StructuredBuffer<GlyphLocation> glyphs_atlas_map;

struct GpuBuffersGlyphStorage : IGlyphsStorage {
    static func texture_to_world_scale() -> float {
        return uniforms.glyphs_atlas_texel_to_world_scale;
    }

    static func locate_glyph(uint glyphIndex) -> GlyphLocation {
        return glyphs_atlas_map[glyphIndex];
    }

    static func sample(float2 uv) -> float3 {
        return glyphs_atlas.SampleLevel(glyphs_atlas_sampler, uv, 0).rgb;
    }
}

export public func sdf_glyph_gpu_hosted(glyph_index: uint32_t, position: float3) -> float {
    return sdf_glyph<GpuBuffersGlyphStorage>(glyph_index, position);
}