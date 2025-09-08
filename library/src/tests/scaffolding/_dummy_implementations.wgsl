
fn procedural_texture_select(index: i32, position: vec3f, normal: vec3f, time: f32, dp_dx: vec3f, dp_dy: vec3f) -> vec3f {
    return vec3f(0.0);
}

fn sdf_select(index: i32, position: vec3f, time: f32) -> f32 {
    return 0.0;
}

fn sdf_apply_animation(index: i32, position: vec3f, time: f32) -> vec3f {
    return vec3f(0.0);
}