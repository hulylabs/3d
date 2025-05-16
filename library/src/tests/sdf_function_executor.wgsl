@group(0) @binding( 0) var<storage, read> input_points: array<vec4f>;
@group(0) @binding( 1) var<storage, read_write> output_values: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&input_points)) {
        return;
    }
    let argument = input_points[index];
    output_values[index] = sdf_select(argument.w, argument.xyz);
}
