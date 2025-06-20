@group(3) @binding( 0) var<storage, read> input: array<Vertex>;
@group(3) @binding( 1) var<storage, read_write> output: array<vec4<f32>>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&input)) {
        return;
    }
    let argument = input[index];
    output[index] = shade_vertex(argument.position.xyz);
}

struct Vertex {
     position: vec3<f32>, 
}

struct Material {
     diffuse: vec3<f32>,  shininess: f32, 
}


SOME ADDITIONAL CODE 1

SOME ADDITIONAL CODE 2
