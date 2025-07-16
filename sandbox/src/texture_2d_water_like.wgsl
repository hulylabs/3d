fn water_hash_u32(x: u32) -> u32 {
    var x_mut = x;
    x_mut += (x_mut << 10u);
    x_mut ^= (x_mut >> 6u);
    x_mut += (x_mut << 3u);
    x_mut ^= (x_mut >> 11u);
    x_mut += (x_mut << 15u);
    return x_mut;
}

fn water_hash_vec2u(v: vec2<u32>) -> u32 {
    return water_hash_u32(v.x ^ water_hash_u32(v.y));
}

fn water_float_construct(m: u32) -> f32 {
    let ieee_mantissa: u32 = 0x007FFFFFu; // binary32 mantissa bitmask
    let ieee_one: u32 = 0x3F800000u;      // 1.0 in IEEE binary32
    var m_mut = m;
    m_mut &= ieee_mantissa;               // Keep only mantissa bits (fractional part)
    m_mut |= ieee_one;                    // Add fractional part to 1.0
    let f = bitcast<f32>(m_mut);          // Range [1:2]
    return f - 1.0;                       // Range [0:1]
}

fn water_random(v: vec2<f32>) -> f32 {
    return water_float_construct(water_hash_vec2u(bitcast<vec2<u32>>(v)));
}

fn water_noise(uv: vec2<f32>) -> f32 {
    let i = floor(uv);
    let f = fract(uv);

    // Four corners in 2D of a tile
    let a = water_random(i);
    let b = water_random(i + vec2<f32>(1.0, 0.0));
    let c = water_random(i + vec2<f32>(0.0, 1.0));
    let d = water_random(i + vec2<f32>(1.0, 1.0));

    // Smooth Interpolation
    // Cubic Hermine Curve. Same as SmoothStep()
    let u = f * f * (3.0 - 2.0 * f);
    // u = smoothstep(0.0, 1.0, f);

    // Mix 4 corners percentages
    return mix(a, b, u.x) +
           (c - a) * u.y * (1.0 - u.x) +
           (d - b) * u.x * u.y;
}

fn water_noise_detailed(uv: vec2<f32>, detail: f32) -> f32 {
    var n = 0.0;
    var m = 0.0;

    for (var i = 0.0; i < detail; i += 1.0) {
        let x = pow(2.0, i);
        let y = 1.0 / x;

        n += water_noise(uv * x + y) * y;
        m += y;
    }

    return n / m;
}

fn water_rot(a: f32) -> mat2x2<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat2x2<f32>(c, -s, s, c);
}

// https://www.shadertoy.com/view/WdXyDj

fn water_like_surface(uv: vec2<f32>) -> vec3<f32> {
    var uv_scaled = uv * 15.0;

    let i_time: f32 = 0.0;

    let n = water_noise_detailed(uv_scaled, 7.0);
    uv_scaled *= water_rot(n + i_time * 0.01) * n;

    let col = mix(
        vec3<f32>(0.0, 0.0, 0.1),
        vec3<f32>(0.0, 1.0, 1.0),
        water_noise_detailed(uv_scaled + i_time * vec2<f32>(0.0, 0.6), 2.0)
    );

    return col;
}