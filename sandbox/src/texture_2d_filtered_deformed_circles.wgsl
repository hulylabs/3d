fn manual_fwidth(dx: vec3f, dy: vec3f) -> vec3f {
    return abs(dx) + abs(dy);
}

// Filtered cosine function using manual gradients
fn fcos_manual(x: vec3f, dx: vec3f, dy: vec3f) -> vec3f {
    let w = manual_fwidth(dx, dy);

    // Choose your filtering method:
    // Option 1: filtered-exact
    return cos(x) * sin(0.5 * w) / (0.5 * w);

    // Option 2: filtered-approx (current implementation)
    //return cos(x) * smoothstep(vec3f(6.28318), vec3f(0.0), w);
}

fn get_color_manual(t: f32, dt_dx: f32, dt_dy: f32) -> vec3f {
    var col = vec3f(0.4, 0.4, 0.4);

    // Term 1: freq=1.0, amp=0.12, phase=(0.0,0.8,1.1)
    var freq = 1.0;
    var amp = 0.12;
    var phase = vec3f(0.0, 0.8, 1.1);
    var arg = 6.28318 * t * freq + phase;
    var darg_dx = vec3f(6.28318 * dt_dx * freq);
    var darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 2: freq=3.1, amp=0.11, phase=(0.3,0.4,0.1)
    freq = 3.1;
    amp = 0.11;
    phase = vec3f(0.3, 0.4, 0.1);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 3: freq=5.1, amp=0.10, phase=(0.1,0.7,1.1)
    freq = 5.1;
    amp = 0.10;
    phase = vec3f(0.1, 0.7, 1.1);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 4: freq=9.1, amp=0.09, phase=(0.2,0.8,1.4)
    freq = 9.1;
    amp = 0.09;
    phase = vec3f(0.2, 0.8, 1.4);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 5: freq=17.1, amp=0.08, phase=(0.2,0.6,0.7)
    freq = 17.1;
    amp = 0.08;
    phase = vec3f(0.2, 0.6, 0.7);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 6: freq=31.1, amp=0.07, phase=(0.1,0.6,0.7)
    freq = 31.1;
    amp = 0.07;
    phase = vec3f(0.1, 0.6, 0.7);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 7: freq=65.1, amp=0.06, phase=(0.0,0.5,0.8)
    freq = 65.1;
    amp = 0.06;
    phase = vec3f(0.0, 0.5, 0.8);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 8: freq=115.1, amp=0.06, phase=(0.1,0.4,0.7)
    freq = 115.1;
    amp = 0.06;
    phase = vec3f(0.1, 0.4, 0.7);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    // Term 9: freq=265.1, amp=0.09, phase=(1.1,1.4,2.7)
    freq = 265.1;
    amp = 0.09;
    phase = vec3f(1.1, 1.4, 2.7);
    arg = 6.28318 * t * freq + phase;
    darg_dx = vec3f(6.28318 * dt_dx * freq);
    darg_dy = vec3f(6.28318 * dt_dy * freq);
    col += amp * fcos_manual(arg, darg_dx, darg_dy);

    return col;
}

fn deform_with_derivatives(p: vec2<f32>, dp_dx: vec2<f32>, dp_dy: vec2<f32>, time: f32) -> array<vec2<f32>, 3> {
    var q = p;
    var dq_dx = dp_dx;
    var dq_dy = dp_dy;

    // deform 1: q *= 0.25
    q *= 0.25;
    dq_dx *= 0.25;
    dq_dy *= 0.25;

    // q = 0.5 * q / dot(q, q)
    let dot_q = dot(q, q);
    let inv_dot_q = 1.0 / dot_q;
    let inv_dot_q_sq = inv_dot_q * inv_dot_q;

    let ddot_dx = 2.0 * dot(q, dq_dx);
    let ddot_dy = 2.0 * dot(q, dq_dy);

    let temp_dq_dx = 0.5 * (dq_dx * inv_dot_q - q * ddot_dx * inv_dot_q_sq);
    let temp_dq_dy = 0.5 * (dq_dy * inv_dot_q - q * ddot_dy * inv_dot_q_sq);

    q = 0.5 * q * inv_dot_q;
    q.x += time * 0.1;
    dq_dx = temp_dq_dx;
    dq_dy = temp_dq_dy;

    // deform 2: Apply each cosine term
    // Note: WGSL doesn't support array initialization like this, so we'll handle each term separately

    // Term 1: coeff=1.5, time_coeff=0.03*1.0, offset=(0.1,1.1)
    var coeff = 1.5;
    var time_coeff = 0.03 * 1.0;
    var offset = vec2<f32>(0.1, 1.1);
    var arg = coeff * q.yx + time_coeff * time + offset;
    var sin_arg = sin(arg);
    var darg_dx = coeff * vec2<f32>(dq_dx.y, dq_dx.x);
    var darg_dy = coeff * vec2<f32>(dq_dy.y, dq_dy.x);
    dq_dx += 0.2 * (-sin_arg * darg_dx);
    dq_dy += 0.2 * (-sin_arg * darg_dy);
    q += 0.2 * cos(arg);

    // Term 2: coeff=2.4, time_coeff=0.03*1.6, offset=(4.5,2.6)
    coeff = 2.4;
    time_coeff = 0.03 * 1.6;
    offset = vec2<f32>(4.5, 2.6);
    arg = coeff * q.yx + time_coeff * time + offset;
    sin_arg = sin(arg);
    darg_dx = coeff * vec2<f32>(dq_dx.y, dq_dx.x);
    darg_dy = coeff * vec2<f32>(dq_dy.y, dq_dy.x);
    dq_dx += 0.2 * (-sin_arg * darg_dx);
    dq_dy += 0.2 * (-sin_arg * darg_dy);
    q += 0.2 * cos(arg);

    // Term 3: coeff=3.3, time_coeff=0.03*1.2, offset=(3.2,3.4)
    coeff = 3.3;
    time_coeff = 0.03 * 1.2;
    offset = vec2<f32>(3.2, 3.4);
    arg = coeff * q.yx + time_coeff * time + offset;
    sin_arg = sin(arg);
    darg_dx = coeff * vec2<f32>(dq_dx.y, dq_dx.x);
    darg_dy = coeff * vec2<f32>(dq_dy.y, dq_dy.x);
    dq_dx += 0.2 * (-sin_arg * darg_dx);
    dq_dy += 0.2 * (-sin_arg * darg_dy);
    q += 0.2 * cos(arg);

    // Term 4: coeff=4.2, time_coeff=0.03*1.7, offset=(1.8,5.2)
    coeff = 4.2;
    time_coeff = 0.03 * 1.7;
    offset = vec2<f32>(1.8, 5.2);
    arg = coeff * q.yx + time_coeff * time + offset;
    sin_arg = sin(arg);
    darg_dx = coeff * vec2<f32>(dq_dx.y, dq_dx.x);
    darg_dy = coeff * vec2<f32>(dq_dy.y, dq_dy.x);
    dq_dx += 0.2 * (-sin_arg * darg_dx);
    dq_dy += 0.2 * (-sin_arg * darg_dy);
    q += 0.2 * cos(arg);

    // Term 5: coeff=9.1, time_coeff=0.03*1.1, offset=(6.3,3.9)
    coeff = 9.1;
    time_coeff = 0.03 * 1.1;
    offset = vec2<f32>(6.3, 3.9);
    arg = coeff * q.yx + time_coeff * time + offset;
    sin_arg = sin(arg);
    darg_dx = coeff * vec2<f32>(dq_dx.y, dq_dx.x);
    darg_dy = coeff * vec2<f32>(dq_dy.y, dq_dy.x);
    dq_dx += 0.2 * (-sin_arg * darg_dx);
    dq_dy += 0.2 * (-sin_arg * darg_dy);
    q += 0.2 * cos(arg);

    return array<vec2<f32>, 3>(q, dq_dx, dq_dy);
}

fn adjust_contrast(color: vec3f, contrast: f32) -> vec3f {
    return (color - vec3f(0.5)) * contrast + vec3f(0.5);
}

// https://www.shadertoy.com/view/wtXfRH

fn deformed_circles_texture(uv: vec2<f32>, time: f32, ray_diffs_dp_dx: vec2f, ray_diffs_dp_dy: vec2f) -> vec3f {
    let uv_scaled = 2.0 * uv;
    let dp_dx = 2.0 * ray_diffs_dp_dx;
    let dp_dy = 2.0 * ray_diffs_dp_dy;

    // deformation with derivatives
    let deformation = deform_with_derivatives(uv_scaled, dp_dx, dp_dy, 0.0);
    let uv_deformed = deformation[0];
    let dp_deformed_dx = deformation[1];
    let dp_deformed_dy = deformation[2];

    // Calculate t = 0.5 * length(p_deformed) and its derivatives
    let half_uv_length = length(uv_deformed);
    let t = 0.5 * half_uv_length;

    // Derivative of length
    let dt_dx = 0.5 * dot(uv_deformed, dp_deformed_dx) / half_uv_length;
    let dt_dy = 0.5 * dot(uv_deformed, dp_deformed_dy) / half_uv_length;

    // Get base color pattern with gradients
    var color = get_color_manual(t, dt_dx, dt_dy);
    color = adjust_contrast(color, 3.0);

    return color;
}