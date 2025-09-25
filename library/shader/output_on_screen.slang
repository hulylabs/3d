#language slang 2026

module output_on_screen;

// final image output (aka resolve): tone mapping + gamma correction

import "output_on_screen_vertex";
import "output_on_screen_pixel";
import "uniforms";
import "tracing_output";

public struct VSOutput {
    float4 position : SV_Position;
};

[shader("fragment")]
export public float4 fs(VSOutput input) : SV_Target0 {
    uint i = pixel_global_index(input.position.xy, uniforms.frame_buffer_size.x);
    float3 color = pixel_color_buffer[i].xyz / uniforms.frame_number;

    color = aces_approx(color.xyz);
    color = pow(color.xyz, float3(1.0 / 2.2));
    color = pseudo_dither(color, input.position.xy);

    return float4(color, 1.0);
}

[shader("vertex")]
export public VSOutput vs(uint in_vertex_index : SV_VertexID) {
    VSOutput output;
    output.position = float4(full_screen_quad_positions[in_vertex_index], 0.0, 1.0);
    return output;
}
