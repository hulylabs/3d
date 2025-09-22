#language slang 2026

implementing output_on_screen;

static const float2 full_screen_quad_positions[6] = {
    float2(-1.0, -1.0),
    float2 (1.0, -1.0),
    float2(-1.0,  1.0),

    float2(-1.0,  1.0),
    float2( 1.0, -1.0),
    float2( 1.0,  1.0),
};

public struct VSOutput {
    float4 position : SV_Position;
};

[shader("vertex")]
export public VSOutput vs(uint in_vertex_index : SV_VertexID) {
    VSOutput output;
    output.position = float4(full_screen_quad_positions[in_vertex_index], 0.0, 1.0);
    return output;
}
