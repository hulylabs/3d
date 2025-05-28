@group({binding_group}) @binding( 0) var<storage, read> input: array<{ input_type }>;
@group({binding_group}) @binding( 1) var<storage, read_write> output: array<{ output_type }>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) \{
    let index = global_id.x;
    if (index >= arrayLength(&input)) \{
        return;
    }
    let argument = input[index];
    output[index] = { function_name }({ argument | argument_deconstructor });
}
{{ for type_declaration in custom_types }}
struct { type_declaration.name } \{
    {{ for field in type_declaration.fields }} { field.name }: { field.type }, {{ endfor }}
}
{{ endfor }}
{{ for code in additional_code }}
{ code }
{{ endfor }}