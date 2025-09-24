#language slang 2026

module uniforms;

[vk::binding(0, 0)]
public ConstantBuffer<Uniforms> uniforms;

public struct Uniforms {
    public uint2 frame_buffer_size;
    public uint frame_buffer_area;
    public float frame_buffer_aspect; // width / height
    
    public float2 inverted_frame_buffer_size;
    public float frame_number;
    private float empty_slot__1;
    
    public float4 view_matrix_col_0;
    public float4 view_matrix_col_1;
    public float4 view_matrix_col_2;
    public float4 view_matrix_col_3;
    /* Consider a view ray defined by an origin (e.g., the eye position for a perspective camera)
    and a direction that intersects the view plane at a world-space pixel position.
    This matrix, when multiplied by the world-space pixel position, returns the ray's origin.
    For a perspective camera, the origin is always the eye position â€" the same for all pixels.
    For an orthographic camera, the origin lies on the camera plane and varies per pixel. */
    public float4 view_ray_origin_matrix_col_0;
    public float4 view_ray_origin_matrix_col_1;
    public float4 view_ray_origin_matrix_col_2;
    public float4 view_ray_origin_matrix_col_3;

    public uint parallelograms_count;
    public uint bvh_length;
    public uint pixel_side_subdivision; // anti-aliasing level: bigger value -> slower render -> less jagged edges
    public float global_time_seconds;

    public uint3 thread_grid_size;
    private float empty_slot__2;
};
