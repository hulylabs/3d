fn lava_permute(x: vec3<f32>) -> vec3<f32> {
    return ((x * 34.0 + 1.0) * x) % 289.0;
}

fn lava_noise(v: vec2<f32>) -> f32 {
    const C = vec4<f32>(0.211324865405187, 0.366025403784439, -0.577350269189626, 0.024390243902439);
    let i = floor(v + dot(v, C.yy));
    let x0 = v - i + dot(i, C.xx);

    var i1: vec2<f32>;
    if (x0.x > x0.y) {
        i1 = vec2<f32>(1.0, 0.0);
    } else {
        i1 = vec2<f32>(0.0, 1.0);
    }

    var x12 = x0.xyxy + C.xxzz;
    x12 = vec4<f32>(x12.xy - i1, x12.zw);

    let i_mod = i % 289.0;
    let p = lava_permute(lava_permute(i_mod.y + vec3<f32>(0.0, i1.y, 1.0)) + i_mod.x + vec3<f32>(0.0, i1.x, 1.0));

    var m = max(vec3<f32>(0.5) - vec3<f32>(dot(x0, x0), dot(x12.xy, x12.xy), dot(x12.zw, x12.zw)), vec3<f32>(0.0));
    m = m * m;
    m = m * m;

    let x_vals = 2.0 * fract(p * C.www) - 1.0;
    let h = abs(x_vals) - 0.5;
    let ox = floor(x_vals + 0.5);
    let a0 = x_vals - ox;

    m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);

    var g: vec3<f32>;
    g.x = a0.x * x0.x + h.x * x0.y;
    g.y = a0.y * x12.x + h.y * x12.y;
    g.z = a0.z * x12.z + h.z * x12.w;

    return 130.0 * dot(m, g);
}

fn lava_fbm(x_input: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var x = x_input;
    let shift = vec2<f32>(100.0);

    const rot = mat2x2<f32>(cos(0.5), sin(0.5), -sin(0.5), cos(0.5));
    const NUM_OCTAVES = 6;

    // Note: NUM_OCTAVES needs to be defined as a constant
    for (var i = 0; i < NUM_OCTAVES; i++) {
        v += a * lava_noise(x);
        x = rot * x * 2.0 + shift;
        a *= 0.5;
    }

    return v;
}

// https://www.shadertoy.com/view/ltdcD7

fn lava_like_texture(uv: vec2<f32>, time: f32, dp_dx: vec2f, dp_dy: vec2f) -> vec3<f32> {
    let scaled_time = time / 10.0;
    let f = lava_fbm(vec2<f32>(scaled_time) + uv + lava_fbm(vec2<f32>(scaled_time) - uv));

    var r = smoothstep(0.0, 0.4, f);
    var g = smoothstep(0.3, 0.7, f);
    var b = smoothstep(0.6, 1.0, f);

    let col = vec3<f32>(r, g, b);
    let f2 = 0.5 - f;

    r = smoothstep(0.0, 0.6, f2);
    g = smoothstep(0.3, 0.9, f2);
    b = smoothstep(0.4, 1.0, f2);

    let col2 = vec3<f32>(r, g, b);
    return mix(col, col2, f2);
}