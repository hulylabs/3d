#language slang 2026

module glyphs_storage;

public struct GlyphLocation {
    float2 top_left_corner_uv;
    float2 size;

    public __init(float2 top_left_corner_uv, float2 size) {
        this.top_left_corner_uv = top_left_corner_uv;
        this.size = size;
    }

    public func evaluate_uv(position: float2) -> float2 {
        let uv_local = position * float2(1.0, -1.0) + this.size * 0.5;
        let uv_absolute = this.top_left_corner_uv + uv_local;
        let uv_clamped = clamp(uv_absolute, this.top_left_corner_uv, this.top_left_corner_uv + this.size);
        return uv_clamped;
    }
}

public interface IGlyphsStorage {
    static func texture_to_world_scale() -> float;
    static func locate_glyph(glyph_index: int32_t) -> GlyphLocation;
    static func sample(uv: float2) -> float3;
}
