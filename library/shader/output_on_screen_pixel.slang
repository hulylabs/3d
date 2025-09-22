#language slang 2026

implementing output_on_screen;

import "uniforms";
import "tracing_output";

// ACES approximation for tone mapping
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/:
float3 aces_approx(float3 v) {
    float3 v1 = v * 0.6f; // TODO: WTF?
    static const float a = 2.51f;
    static const float b = 0.03f;
    static const float c = 2.43f;
    static const float d = 0.59f;
    static const float e = 0.14f;
    return clamp((v1*(a*v1+b))/(v1*(c*v1+d)+e), float3(0.0f), float3(1.0f));
}

uint pixel_global_index(float2 pixel_position) {
    return uint(pixel_position.y) * uniforms.frame_buffer_size.x + uint(pixel_position.x);
}

/*
Gradient noise from Jorge Jimenez's presentation:
Next Generation Post Processing in Call of Duty: Advanced Warfare
http://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare

Good post on usage: https://blog.frost.kiwi/GLSL-noise-and-radial-gradient/
*/
float gradient_noise(float2 uv) {
    return frac(52.9829189 * frac(dot(uv, float2(0.06711056, 0.00583715))));
}

float3 pseudo_dither(float3 color, float2 pixel_coordinate) {
    return color + (1.0 / 255.0) * gradient_noise(pixel_coordinate) - (0.5 / 255.0);
}

[shader("fragment")]
export public float4 fs(VSOutput input) : SV_Target0 {
    uint i = pixel_global_index(input.position.xy);
    float3 color = pixel_color_buffer[i].xyz / uniforms.frame_number;

    color = aces_approx(color.xyz);
    color = pow(color.xyz, float3(1.0 / 2.2));
    color = pseudo_dither(color, input.position.xy);

    return float4(color, 1.0);
}
