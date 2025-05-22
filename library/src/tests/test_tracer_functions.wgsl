struct PositionAndDirection {
    position: vec4f,
    direction: vec4f,
} 

@group(3) @binding( 0) var<storage, read> input_points: array<PositionAndDirection>;
@group(3) @binding( 1) var<storage, read_write> output_values: array<RayMarchStep>;

@compute @workgroup_size(64, 1, 1)
fn test_entry_point(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&input_points)) {
        return;
    }
    let argument = input_points[index];
    output_values[index] = sample_signed_distance(argument.position.xyz, argument.direction.xyz);
}
