#language slang 2026

module tracing_output;

[vk::binding(0, 1)] public RWStructuredBuffer<float4> pixel_color_buffer;
[vk::binding(1, 1)] public RWStructuredBuffer<uint  > object_id_buffer;
[vk::binding(2, 1)] public RWStructuredBuffer<float4> normal_buffer;
[vk::binding(3, 1)] public RWStructuredBuffer<float4> albedo_buffer;
