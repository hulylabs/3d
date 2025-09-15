struct SLANG_ParameterGroup_uniforms_std140_0
{
    frame_buffer_size : vec2<u32>,
    frame_buffer_area : u32,
    frame_buffer_aspect : f32,
    inverted_frame_buffer_size : vec2<f32>,
    frame_number : f32,
    empty_slot__1 : f32,
    view_matrix_col_0 : vec4<f32>,
    view_matrix_col_1 : vec4<f32>,
    view_matrix_col_2 : vec4<f32>,
    view_matrix_col_3 : vec4<f32>,
    view_ray_origin_matrix_col_0 : vec4<f32>,
    view_ray_origin_matrix_col_1 : vec4<f32>,
    view_ray_origin_matrix_col_2 : vec4<f32>,
    view_ray_origin_matrix_col_3 : vec4<f32>,
    parallelograms_count : u32,
    bvh_length : u32,
    pixel_side_subdivision : u32,
    global_time_seconds : f32,
    thread_grid_size : vec3<u32>,
    empty_slot__2 : f32,
};

@binding(0) @group(0) var<uniform> uniforms : SLANG_ParameterGroup_uniforms_std140_0;
struct Parallelogram_std430_0
{
    Q_0 : vec3<f32>,
    u_0 : vec3<f32>,
    object_uid_0 : u32,
    v_0 : vec3<f32>,
    D_0 : f32,
    normal_0 : vec3<f32>,
    w_0 : vec3<f32>,
    material_id_0 : u32,
};

@binding(0) @group(2) var<storage, read> parallelograms : array<Parallelogram_std430_0>;

struct BvhNode_std430_0
{
    aabb_min_0 : vec3<f32>,
    primitive_index_0 : u32,
    aabb_max_0 : vec3<f32>,
    primitive_type_0 : u32,
    hit_miss_skip_link_0 : i32,
};

@binding(4) @group(2) var<storage, read> bvh : array<BvhNode_std430_0>;

struct Triangle_std430_0
{
    A_0 : vec3<f32>,
    B_0 : vec3<f32>,
    C_0 : vec3<f32>,
    normalA_0 : vec3<f32>,
    normalB_0 : vec3<f32>,
    object_uid_1 : u32,
    normalC_0 : vec3<f32>,
    material_id_1 : u32,
};

@binding(2) @group(2) var<storage, read> triangles : array<Triangle_std430_0>;

struct Sdf_std430_0
{
    location_col_0_0 : vec4<f32>,
    location_col_1_0 : vec4<f32>,
    location_col_2_0 : vec4<f32>,
    inverse_location_col_0_0 : vec4<f32>,
    inverse_location_col_1_0 : vec4<f32>,
    inverse_location_col_2_0 : vec4<f32>,
    ray_marching_step_scale_0 : f32,
    class_index_0 : i32,
    material_id_2 : u32,
    object_uid_2 : u32,
};

@binding(1) @group(2) var<storage, read> sdf : array<Sdf_std430_0>;

@binding(6) @group(2) var<storage, read> sdf_time : array<f32>;

struct Material_std430_0
{
    albedo_0 : vec3<f32>,
    emission_0 : vec3<f32>,
    specular_0 : vec3<f32>,
    specular_strength_0 : f32,
    roughness_0 : f32,
    refractive_index_eta_0 : f32,
    albedo_texture_uid_0 : i32,
    material_class_0 : i32,
};

@binding(3) @group(2) var<storage, read> materials : array<Material_std430_0>;

struct AtlasMapping_std430_0
{
    top_left_corner_uv_0 : vec2<f32>,
    size_0 : vec2<f32>,
    local_position_to_texture_x_0 : vec4<f32>,
    local_position_to_texture_y_0 : vec4<f32>,
    wrap_mode_0 : vec2<i32>,
};

@binding(7) @group(2) var<storage, read> texture_atlases_mapping : array<AtlasMapping_std430_0>;

@binding(2) @group(0) var texture_atlas_page : texture_2d<f32>;

@binding(1) @group(0) var atlases_sampler : sampler;

@binding(1) @group(1) var<storage, read_write> object_id_buffer : array<u32>;

@binding(3) @group(1) var<storage, read_write> albedo_buffer : array<vec4<f32>>;

@binding(2) @group(1) var<storage, read_write> normal_buffer : array<vec4<f32>>;

@binding(0) @group(1) var<storage, read_write> pixel_color_buffer : array<vec4<f32>>;

@binding(5) @group(2) var<storage, read> bvh_inflated : array<BvhNode_std430_0>;

const full_screen_quad_positions_0 : array<vec2<f32>, i32(6)> = array<vec2<f32>, i32(6)>( vec2<f32>(-1.0f, -1.0f), vec2<f32>(1.0f, -1.0f), vec2<f32>(-1.0f, 1.0f), vec2<f32>(-1.0f, 1.0f), vec2<f32>(1.0f, -1.0f), vec2<f32>(1.0f, 1.0f) );
fn evaluate_pixel_index_0( global_invocation_id_0 : vec3<u32>,  thread_grid_size_0 : vec3<u32>) -> u32
{
    var _S1 : u32 = thread_grid_size_0.x;
    return global_invocation_id_0.z * (_S1 * thread_grid_size_0.y) + global_invocation_id_0.y * _S1 + global_invocation_id_0.x;
}

fn pixel_outside_frame_buffer_0( pixel_index_0 : u32) -> bool
{
    return pixel_index_0 >= (uniforms.frame_buffer_area);
}

struct Pixel_0
{
     coordinates_0 : vec2<f32>,
};

fn setup_pixel_coordinates_0( pixel_index_1 : u32) -> Pixel_0
{
    var _S2 : u32 = uniforms.frame_buffer_size.x;
    var x_0 : u32 = pixel_index_1 % _S2;
    var y_0 : u32 = pixel_index_1 / _S2;
    var result_0 : Pixel_0;
    result_0.coordinates_0 = vec2<f32>(f32(x_0), f32(y_0));
    return result_0;
}

struct Camera_0
{
     fov_factor_0 : f32,
     origin_0 : vec3<f32>,
};

fn setup_camera_0() -> Camera_0
{
    var origin_1 : vec3<f32> = uniforms.view_matrix_col_3.xyz;
    var result_1 : Camera_0;
    result_1.fov_factor_0 = 1.0f / tan(0.52359879016876221f);
    result_1.origin_0 = origin_1;
    return result_1;
}

struct Ray_0
{
     origin_2 : vec3<f32>,
     direction_0 : vec3<f32>,
};

fn get_camera_ray_0( camera_0 : Camera_0,  s_0 : f32,  t_0 : f32) -> Ray_0
{
    var pixel_world_space_0 : vec4<f32> = vec4<f32>(camera_0.origin_0 + (((mat4x4<f32>(uniforms.view_matrix_col_0, uniforms.view_matrix_col_1, uniforms.view_matrix_col_2, uniforms.view_matrix_col_3)) * (vec4<f32>(vec3<f32>(s_0, t_0, - camera_0.fov_factor_0), 0.0f)))).xyz, 1.0f);
    var ray_origin_world_space_0 : vec3<f32> = (((mat4x4<f32>(uniforms.view_ray_origin_matrix_col_0, uniforms.view_ray_origin_matrix_col_1, uniforms.view_ray_origin_matrix_col_2, uniforms.view_ray_origin_matrix_col_3)) * (pixel_world_space_0))).xyz;
    var direction_1 : vec3<f32> = normalize(pixel_world_space_0.xyz - ray_origin_world_space_0);
    var result_2 : Ray_0;
    result_2.origin_2 = ray_origin_world_space_0;
    result_2.direction_0 = direction_1;
    return result_2;
}

fn ray_to_pixel_0( camera_1 : Camera_0,  pixel_0 : Pixel_0,  sub_pixel_x_0 : f32,  sub_pixel_y_0 : f32) -> Ray_0
{
    return get_camera_ray_0(camera_1, uniforms.frame_buffer_aspect * (2.0f * ((pixel_0.coordinates_0.x + sub_pixel_x_0) * uniforms.inverted_frame_buffer_size.x) - 1.0f), -1.0f * (2.0f * ((pixel_0.coordinates_0.y + sub_pixel_y_0) * uniforms.inverted_frame_buffer_size.y) - 1.0f));
}

struct RayDifferentials_0
{
     dx_0 : vec3<f32>,
     dy_0 : vec3<f32>,
};

fn ray_differentials_0( camera_2 : Camera_0,  pixel_1 : Pixel_0,  sub_pixel_x_1 : f32,  sub_pixel_y_1 : f32) -> RayDifferentials_0
{
    var pixel_dx_0 : Pixel_0;
    pixel_dx_0.coordinates_0 = pixel_1.coordinates_0 + vec2<f32>(1.0f, 0.0f);
    var ray_direction_dx_0 : Ray_0 = ray_to_pixel_0(camera_2, pixel_dx_0, sub_pixel_x_1, sub_pixel_y_1);
    var pixel_dy_0 : Pixel_0;
    pixel_dy_0.coordinates_0 = pixel_1.coordinates_0 + vec2<f32>(0.0f, 1.0f);
    var ray_direction_dy_0 : Ray_0 = ray_to_pixel_0(camera_2, pixel_dy_0, sub_pixel_x_1, sub_pixel_y_1);
    var result_3 : RayDifferentials_0;
    result_3.dx_0 = ray_direction_dx_0.direction_0;
    result_3.dy_0 = ray_direction_dy_0.direction_0;
    return result_3;
}

struct RayAndDifferentials_0
{
     ray_0 : Ray_0,
     differentials_0 : RayDifferentials_0,
};

fn ray_and_differentials_0( camera_3 : Camera_0,  pixel_2 : Pixel_0,  sub_pixel_x_2 : f32,  sub_pixel_y_2 : f32) -> RayAndDifferentials_0
{
    var differentials_1 : RayDifferentials_0 = ray_differentials_0(camera_3, pixel_2, sub_pixel_x_2, sub_pixel_y_2);
    var result_4 : RayAndDifferentials_0;
    result_4.ray_0 = ray_to_pixel_0(camera_3, pixel_2, sub_pixel_x_2, sub_pixel_y_2);
    result_4.differentials_0 = differentials_1;
    return result_4;
}

fn at_0( ray_1 : Ray_0,  t_1 : f32) -> vec3<f32>
{
    return ray_1.origin_2 + vec3<f32>(t_1) * ray_1.direction_0;
}

struct HitPlace_0
{
     position_0 : vec3<f32>,
     normal_1 : vec3<f32>,
};

struct HitRecord_0
{
     global_0 : HitPlace_0,
     local_0 : HitPlace_0,
     t_2 : f32,
     material_id_3 : u32,
     front_face_0 : bool,
};

var<private> hitRec : HitRecord_0;

struct Parallelogram_0
{
    Q_0 : vec3<f32>,
    u_0 : vec3<f32>,
    object_uid_0 : u32,
    v_0 : vec3<f32>,
    D_0 : f32,
    normal_0 : vec3<f32>,
    w_0 : vec3<f32>,
    material_id_0 : u32,
};

fn hit_quad_0( quad_0 : Parallelogram_0,  tmin_0 : f32,  tmax_0 : f32,  ray_2 : Ray_0) -> bool
{
    if((dot(ray_2.direction_0, quad_0.normal_0)) > 0.0f)
    {
        return false;
    }
    var denom_0 : f32 = dot(quad_0.normal_0, ray_2.direction_0);
    if((abs(denom_0)) < 9.99999993922529029e-09f)
    {
        return false;
    }
    var t_3 : f32 = (quad_0.D_0 - dot(quad_0.normal_0, ray_2.origin_2)) / denom_0;
    var _S3 : bool;
    if(t_3 <= tmin_0)
    {
        _S3 = true;
    }
    else
    {
        _S3 = t_3 >= tmax_0;
    }
    if(_S3)
    {
        return false;
    }
    var planar_hitpt_vector_0 : vec3<f32> = at_0(ray_2, t_3) - quad_0.Q_0;
    var alpha_0 : f32 = dot(quad_0.w_0, cross(planar_hitpt_vector_0, quad_0.v_0));
    var beta_0 : f32 = dot(quad_0.w_0, cross(quad_0.u_0, planar_hitpt_vector_0));
    if(alpha_0 < 0.0f)
    {
        _S3 = true;
    }
    else
    {
        _S3 = 1.0f < alpha_0;
    }
    if(_S3)
    {
        _S3 = true;
    }
    else
    {
        _S3 = beta_0 < 0.0f;
    }
    if(_S3)
    {
        _S3 = true;
    }
    else
    {
        _S3 = 1.0f < beta_0;
    }
    if(_S3)
    {
        return false;
    }
    hitRec.t_2 = t_3;
    var local_position_0 : vec3<f32> = quad_0.u_0 * vec3<f32>(alpha_0) + quad_0.v_0 * vec3<f32>(beta_0);
    hitRec.local_0.position_0 = local_position_0 - (quad_0.u_0 + quad_0.v_0) * vec3<f32>(0.5f);
    hitRec.global_0.position_0 = quad_0.Q_0 + local_position_0;
    hitRec.global_0.normal_1 = quad_0.normal_0;
    var _S4 : bool = denom_0 < 0.0f;
    hitRec.front_face_0 = _S4;
    if(false == _S4)
    {
        hitRec.global_0.normal_1 = (vec3<f32>(0) - hitRec.global_0.normal_1);
    }
    hitRec.local_0.normal_1 = hitRec.global_0.normal_1;
    hitRec.material_id_3 = quad_0.material_id_0;
    return true;
}

struct AabbHit_0
{
     hit_0 : bool,
     ray_parameter_0 : f32,
};

fn hit_aabb_0( box_min_0 : vec3<f32>,  box_max_0 : vec3<f32>,  tmin_1 : f32,  tmax_1 : f32,  ray_origin_0 : vec3<f32>,  inverted_ray_dir_0 : vec3<f32>) -> AabbHit_0
{
    var t0s_0 : vec3<f32> = (box_min_0 - ray_origin_0) * inverted_ray_dir_0;
    var t1s_0 : vec3<f32> = (box_max_0 - ray_origin_0) * inverted_ray_dir_0;
    var tsmaller_0 : vec3<f32> = min(t0s_0, t1s_0);
    var tbigger_0 : vec3<f32> = max(t0s_0, t1s_0);
    var t_min_0 : f32 = max(tmin_1, max(tsmaller_0.x, max(tsmaller_0.y, tsmaller_0.z)));
    var result_5 : AabbHit_0;
    result_5.hit_0 = (min(tmax_1, min(tbigger_0.x, min(tbigger_0.y, tbigger_0.z)))) > t_min_0;
    result_5.ray_parameter_0 = t_min_0;
    return result_5;
}

struct Triangle_0
{
    A_0 : vec3<f32>,
    B_0 : vec3<f32>,
    C_0 : vec3<f32>,
    normalA_0 : vec3<f32>,
    normalB_0 : vec3<f32>,
    object_uid_1 : u32,
    normalC_0 : vec3<f32>,
    material_id_1 : u32,
};

fn hit_triangle_0( triangle_0 : Triangle_0,  tmin_2 : f32,  tmax_2 : f32,  ray_3 : Ray_0) -> bool
{
    var AB_0 : vec3<f32> = triangle_0.B_0 - triangle_0.A_0;
    var AC_0 : vec3<f32> = triangle_0.C_0 - triangle_0.A_0;
    var normal_2 : vec3<f32> = cross(AB_0, AC_0);
    var determinant_0 : f32 = - dot(ray_3.direction_0, normal_2);
    if((abs(determinant_0)) < tmin_2)
    {
        return false;
    }
    var ao_0 : vec3<f32> = ray_3.origin_2 - triangle_0.A_0;
    var dao_0 : vec3<f32> = cross(ao_0, ray_3.direction_0);
    var invDet_0 : f32 = 1.0f / determinant_0;
    var dst_0 : f32 = dot(ao_0, normal_2) * invDet_0;
    var u_1 : f32 = dot(AC_0, dao_0) * invDet_0;
    var v_1 : f32 = - dot(AB_0, dao_0) * invDet_0;
    var w_1 : f32 = 1.0f - u_1 - v_1;
    var _S5 : bool;
    if(dst_0 < tmin_2)
    {
        _S5 = true;
    }
    else
    {
        _S5 = dst_0 > tmax_2;
    }
    if(_S5)
    {
        _S5 = true;
    }
    else
    {
        _S5 = u_1 < tmin_2;
    }
    if(_S5)
    {
        _S5 = true;
    }
    else
    {
        _S5 = v_1 < tmin_2;
    }
    if(_S5)
    {
        _S5 = true;
    }
    else
    {
        _S5 = w_1 < tmin_2;
    }
    if(_S5)
    {
        return false;
    }
    hitRec.t_2 = dst_0;
    var _S6 : vec3<f32> = vec3<f32>(w_1);
    var _S7 : vec3<f32> = vec3<f32>(u_1);
    var _S8 : vec3<f32> = vec3<f32>(v_1);
    var _S9 : vec3<f32> = triangle_0.A_0 * _S6 + triangle_0.B_0 * _S7 + triangle_0.C_0 * _S8;
    hitRec.global_0.position_0 = _S9;
    hitRec.local_0.position_0 = _S9;
    var _S10 : vec3<f32> = normalize(triangle_0.normalA_0 * _S6 + triangle_0.normalB_0 * _S7 + triangle_0.normalC_0 * _S8);
    hitRec.global_0.normal_1 = _S10;
    var _S11 : bool = (dot(ray_3.direction_0, _S10)) < 0.0f;
    hitRec.front_face_0 = _S11;
    if(_S11 == false)
    {
        hitRec.global_0.normal_1 = (vec3<f32>(0) - hitRec.global_0.normal_1);
    }
    hitRec.local_0.normal_1 = hitRec.global_0.normal_1;
    hitRec.material_id_3 = triangle_0.material_id_1;
    return true;
}

fn to_mat3x3_0( source_0 : mat3x4<f32>) -> mat3x3<f32>
{
    return mat3x3<f32>(source_0[i32(0)].xyz, source_0[i32(1)].xyz, source_0[i32(2)].xyz);
}

fn transform_point_0( transformation_0 : mat3x4<f32>,  point_0 : vec3<f32>) -> vec3<f32>
{
    return (((vec4<f32>(point_0, 1.0f)) * (transformation_0)));
}

fn transform_vector_0( transformation_1 : mat3x3<f32>,  vector_0 : vec3<f32>) -> vec3<f32>
{
    return (((vector_0) * (transformation_1)));
}

fn Ray_x24init_0( origin_3 : vec3<f32>,  direction_2 : vec3<f32>) -> Ray_0
{
    var _S12 : Ray_0;
    _S12.origin_2 = origin_3;
    _S12.direction_0 = direction_2;
    return _S12;
}

fn transform_ray_parameter_0( transformation_2 : mat3x4<f32>,  ray_4 : Ray_0,  parameter_0 : f32,  transformed_origin_0 : vec3<f32>) -> f32
{
    return length(transform_point_0(transformation_2, at_0(ray_4, parameter_0)) - transformed_origin_0);
}

struct Sdf_0
{
    location_col_0_0 : vec4<f32>,
    location_col_1_0 : vec4<f32>,
    location_col_2_0 : vec4<f32>,
    inverse_location_col_0_0 : vec4<f32>,
    inverse_location_col_1_0 : vec4<f32>,
    inverse_location_col_2_0 : vec4<f32>,
    ray_marching_step_scale_0 : f32,
    class_index_0 : i32,
    material_id_2 : u32,
    object_uid_2 : u32,
};

fn sample_sdf_0( sdf_0 : Sdf_0,  point_1 : vec3<f32>,  time_0 : f32) -> f32
{
    var _S13 : f32 = sdf_select(sdf_0.class_index_0, point_1, time_0);
    return _S13;
}

fn signed_distance_normal_0( sdf_1 : Sdf_0,  point_2 : vec3<f32>,  time_1 : f32) -> vec3<f32>
{
    var e_0 : vec2<f32> = vec2<f32>(1.0f, -1.0f) * vec2<f32>(0.57730001211166382f) * vec2<f32>(0.00050000002374873f);
    var _S14 : vec3<f32> = e_0.xyy;
    var _S15 : f32 = sample_sdf_0(sdf_1, point_2 + _S14, time_1);
    var _S16 : vec3<f32> = _S14 * vec3<f32>(_S15);
    var _S17 : vec3<f32> = e_0.yyx;
    var _S18 : f32 = sample_sdf_0(sdf_1, point_2 + _S17, time_1);
    var _S19 : vec3<f32> = _S16 + _S17 * vec3<f32>(_S18);
    var _S20 : vec3<f32> = e_0.yxy;
    var _S21 : f32 = sample_sdf_0(sdf_1, point_2 + _S20, time_1);
    var _S22 : vec3<f32> = _S19 + _S20 * vec3<f32>(_S21);
    var _S23 : vec3<f32> = e_0.xxx;
    var _S24 : f32 = sample_sdf_0(sdf_1, point_2 + _S23, time_1);
    return normalize(_S22 + _S23 * vec3<f32>(_S24));
}

fn transform_transposed_vector_0( transformation_3 : mat3x3<f32>,  vector_1 : vec3<f32>) -> vec3<f32>
{
    return (((vector_1) * (transpose(transformation_3))));
}

fn apply_animation_0( sdf_2 : Sdf_0,  point_3 : vec3<f32>,  time_2 : f32) -> vec3<f32>
{
    var _S25 : vec3<f32> = sdf_apply_animation(sdf_2.class_index_0, point_3, time_2);
    return _S25;
}

fn hit_sdf_0( sdf_3 : Sdf_0,  time_3 : f32,  ray_5 : Ray_0,  tmin_3 : f32,  tmax_3 : f32) -> bool
{
    var sdf_inverse_location_0 : mat3x4<f32> = mat3x4<f32>(sdf_3.inverse_location_col_0_0, sdf_3.inverse_location_col_1_0, sdf_3.inverse_location_col_2_0);
    var sdf_location_inverse_0 : mat3x3<f32> = to_mat3x3_0(sdf_inverse_location_0);
    var local_ray_origin_0 : vec3<f32> = transform_point_0(sdf_inverse_location_0, ray_5.origin_2);
    var _S26 : Ray_0 = Ray_x24init_0(local_ray_origin_0, normalize(transform_vector_0(sdf_location_inverse_0, ray_5.direction_0)));
    var _S27 : f32 = transform_ray_parameter_0(sdf_inverse_location_0, ray_5, tmin_3, local_ray_origin_0);
    var _S28 : f32 = transform_ray_parameter_0(sdf_inverse_location_0, ray_5, tmax_3, local_ray_origin_0);
    var i_0 : i32 = i32(0);
    var local_t_0 : f32 = _S27;
    for(;;)
    {
        if(i_0 >= i32(120))
        {
            break;
        }
        if(local_t_0 > _S28)
        {
            break;
        }
        var candidate_0 : vec3<f32> = at_0(_S26, local_t_0);
        var signed_distance_0 : f32 = sample_sdf_0(sdf_3, candidate_0, time_3);
        var t_scaled_0 : f32 = 0.00009999999747379f * local_t_0;
        var _S29 : f32 = abs(signed_distance_0);
        if(_S29 < t_scaled_0)
        {
            var _S30 : vec3<f32> = signed_distance_normal_0(sdf_3, candidate_0, time_3);
            hitRec.local_0.normal_1 = _S30;
            hitRec.global_0.normal_1 = normalize(transform_transposed_vector_0(sdf_location_inverse_0, _S30));
            hitRec.global_0.position_0 = transform_point_0(mat3x4<f32>(sdf_3.location_col_0_0, sdf_3.location_col_1_0, sdf_3.location_col_2_0), candidate_0);
            var _S31 : vec3<f32> = apply_animation_0(sdf_3, candidate_0, time_3);
            hitRec.local_0.position_0 = _S31;
            hitRec.t_2 = length(hitRec.global_0.position_0 - ray_5.origin_2);
            var _S32 : f32 = sample_sdf_0(sdf_3, _S26.origin_2, time_3);
            var _S33 : bool = _S32 >= 0.0f;
            hitRec.front_face_0 = _S33;
            if(_S33 == false)
            {
                hitRec.global_0.normal_1 = (vec3<f32>(0) - hitRec.global_0.normal_1);
                hitRec.local_0.normal_1 = (vec3<f32>(0) - hitRec.local_0.normal_1);
            }
            hitRec.material_id_3 = sdf_3.material_id_2;
            return true;
        }
        var local_t_1 : f32 = local_t_0 + max(_S29 * sdf_3.ray_marching_step_scale_0, t_scaled_0);
        i_0 = i_0 + i32(1);
        local_t_0 = local_t_1;
    }
    return false;
}

fn snap_to_grid_0( victim_0 : vec3<f32>,  grid_step_0 : f32) -> vec3<f32>
{
    var _S34 : vec3<f32> = vec3<f32>(grid_step_0);
    return floor((victim_0 - _S34 * vec3<f32>((vec3<i32>(sign((victim_0)))))) / _S34) * _S34;
}

struct RayDerivatives_0
{
     dp_dx_0 : vec3<f32>,
     dp_dy_0 : vec3<f32>,
};

fn ray_hit_position_derivatives_0( ray_direction_0 : vec3<f32>,  surface_intersection_parameter_0 : f32,  surface_normal_0 : vec3<f32>,  ray_differentials_1 : RayDifferentials_0) -> RayDerivatives_0
{
    var _S35 : vec3<f32> = vec3<f32>(dot(ray_direction_0, surface_normal_0));
    var _S36 : vec3<f32> = vec3<f32>(surface_intersection_parameter_0);
    var dp_dy_1 : vec3<f32> = _S36 * (ray_differentials_1.dy_0 * _S35 / vec3<f32>(dot(ray_differentials_1.dy_0, surface_normal_0)) - ray_direction_0);
    var result_6 : RayDerivatives_0;
    result_6.dp_dx_0 = _S36 * (ray_differentials_1.dx_0 * _S35 / vec3<f32>(dot(ray_differentials_1.dx_0, surface_normal_0)) - ray_direction_0);
    result_6.dp_dy_0 = dp_dy_1;
    return result_6;
}

fn calculate_mip_level_0( target_texture_0 : texture_2d<f32>,  ddx_0 : vec2<f32>,  ddy_0 : vec2<f32>) -> u32
{
    var width_0 : u32;
    var height_0 : u32;
    var mip_levels_0 : u32;
    {var dim = textureDimensions((target_texture_0), (u32(0)));((width_0)) = dim.x;((height_0)) = dim.y;((mip_levels_0)) = textureNumLevels((target_texture_0));};
    var texture_size_0 : vec2<f32> = vec2<f32>(f32(width_0), f32(height_0));
    var delta_max_sqr_0 : f32 = max(length(ddx_0 * texture_size_0), length(ddy_0 * texture_size_0));
    if(delta_max_sqr_0 <= 0.0f)
    {
        return u32(0);
    }
    return clamp(u32(0.5f * log2(delta_max_sqr_0)), u32(0), mip_levels_0 - u32(1));
}

fn pixel_half_size_0( target_texture_1 : texture_2d<f32>,  ddx_1 : vec2<f32>,  ddy_1 : vec2<f32>) -> vec2<f32>
{
    var width_1 : u32;
    var height_1 : u32;
    var mip_level_count_0 : u32;
    {var dim = textureDimensions((target_texture_1), (calculate_mip_level_0(target_texture_1, ddx_1, ddy_1)));((width_1)) = dim.x;((height_1)) = dim.y;((mip_level_count_0)) = textureNumLevels((target_texture_1));};
    return vec2<f32>(0.5f) / vec2<f32>(f32(width_1), f32(height_1));
}

struct AtlasMapping_0
{
    top_left_corner_uv_0 : vec2<f32>,
    size_0 : vec2<f32>,
    local_position_to_texture_x_0 : vec4<f32>,
    local_position_to_texture_y_0 : vec4<f32>,
    wrap_mode_0 : vec2<i32>,
};

fn read_atlas_0( local_space_position_0 : vec3<f32>,  atlas_region_mapping_0 : AtlasMapping_0,  differentials_2 : RayDerivatives_0) -> vec4<f32>
{
    var local_position_to_texture_0 : mat2x4<f32> = mat2x4<f32>(atlas_region_mapping_0.local_position_to_texture_x_0, atlas_region_mapping_0.local_position_to_texture_y_0);
    var texture_coordinate_0 : vec2<f32> = (((vec4<f32>(local_space_position_0, 1.0f)) * (local_position_to_texture_0)));
    var ddx_2 : vec2<f32> = (((vec4<f32>(differentials_2.dp_dx_0, 0.0f)) * (local_position_to_texture_0)));
    var ddy_2 : vec2<f32> = (((vec4<f32>(differentials_2.dp_dy_0, 0.0f)) * (local_position_to_texture_0)));
    var i_1 : i32 = i32(0);
    for(;;)
    {
        if(i_1 < i32(2))
        {
        }
        else
        {
            break;
        }
        var coordinate_0 : f32 = texture_coordinate_0[i_1];
        var _S37 : i32 = i_1;
        var _S38 : vec2<f32> = pixel_half_size_0(texture_atlas_page, ddx_2, ddy_2);
        var min_edge_0 : f32 = _S38[i_1] / atlas_region_mapping_0.size_0[i_1];
        var max_edge_0 : f32 = 1.0f - _S38[i_1] / atlas_region_mapping_0.size_0[i_1];
        if(i32(0) == (atlas_region_mapping_0.wrap_mode_0[i_1]))
        {
            texture_coordinate_0[i_1] = fract(coordinate_0);
        }
        else
        {
            if(i32(1) == (atlas_region_mapping_0.wrap_mode_0[_S37]))
            {
                texture_coordinate_0[i_1] = clamp(coordinate_0, min_edge_0, max_edge_0);
            }
            else
            {
                var _S39 : bool;
                if(coordinate_0 < min_edge_0)
                {
                    _S39 = true;
                }
                else
                {
                    _S39 = coordinate_0 > max_edge_0;
                }
                if(_S39)
                {
                    return vec4<f32>(0.0f);
                }
            }
        }
        i_1 = i_1 + i32(1);
    }
    return (textureSampleGrad((texture_atlas_page), (atlases_sampler), (atlas_region_mapping_0.top_left_corner_uv_0 + texture_coordinate_0.xy * atlas_region_mapping_0.size_0), (ddx_2.xy), (ddy_2.xy)));
}

struct Material_0
{
    albedo_0 : vec3<f32>,
    emission_0 : vec3<f32>,
    specular_0 : vec3<f32>,
    specular_strength_0 : f32,
    roughness_0 : f32,
    refractive_index_eta_0 : f32,
    albedo_texture_uid_0 : i32,
    material_class_0 : i32,
};

fn fetch_albedo_0( hit_1 : HitPlace_0,  ray_direction_1 : vec3<f32>,  ray_parameter_1 : f32,  material_0 : Material_0,  differentials_3 : RayDifferentials_0) -> vec3<f32>
{
    var result_7 : vec3<f32> = material_0.albedo_0.xyz;
    var result_8 : vec3<f32>;
    if((material_0.albedo_texture_uid_0) < i32(0))
    {
        var derivartives_0 : RayDerivatives_0 = ray_hit_position_derivatives_0(ray_direction_1, ray_parameter_1, hit_1.normal_1, differentials_3);
        var _S40 : vec3<f32> = procedural_texture_select(- material_0.albedo_texture_uid_0, snap_to_grid_0(hit_1.position_0, 0.00009999999747379f), hit_1.normal_1, uniforms.global_time_seconds, derivartives_0.dp_dx_0, derivartives_0.dp_dy_0);
        result_8 = result_7 * _S40;
    }
    else
    {
        if((material_0.albedo_texture_uid_0) > i32(0))
        {
            var _S41 : AtlasMapping_0 = AtlasMapping_0( texture_atlases_mapping[material_0.albedo_texture_uid_0 - i32(1)].top_left_corner_uv_0, texture_atlases_mapping[material_0.albedo_texture_uid_0 - i32(1)].size_0, texture_atlases_mapping[material_0.albedo_texture_uid_0 - i32(1)].local_position_to_texture_x_0, texture_atlases_mapping[material_0.albedo_texture_uid_0 - i32(1)].local_position_to_texture_y_0, texture_atlases_mapping[material_0.albedo_texture_uid_0 - i32(1)].wrap_mode_0 );
            var texture_sample_0 : vec4<f32> = read_atlas_0(hit_1.position_0, _S41, ray_hit_position_derivatives_0(ray_direction_1, ray_parameter_1, hit_1.normal_1, differentials_3));
            var _S42 : f32 = texture_sample_0.w;
            result_8 = vec3<f32>((1.0f - _S42)) * result_7 + vec3<f32>(_S42) * texture_sample_0.xyz;
        }
        else
        {
            result_8 = result_7;
        }
    }
    return result_8;
}

struct FirstHitSurface_0
{
     object_uid_3 : u32,
     albedo_1 : vec3<f32>,
     normal_3 : vec3<f32>,
};

fn trace_first_intersection_0( incident_0 : RayAndDifferentials_0) -> FirstHitSurface_0
{
    var hit_global_normal_0 : vec3<f32>;
    var hit_material_id_0 : u32;
    var hit_uid_0 : u32;
    var closest_so_far_0 : f32;
    var _S43 : vec3<f32> = vec3<f32>(0.0f);
    var hit_local_0 : HitPlace_0;
    hit_local_0.position_0 = _S43;
    hit_local_0.normal_1 = _S43;
    var closest_so_far_1 : f32 = 1.0e+09f;
    var hit_uid_1 : u32 = u32(0);
    var hit_material_id_1 : u32 = u32(0);
    var hit_global_normal_1 : vec3<f32> = _S43;
    var i_2 : u32 = u32(0);
    for(;;)
    {
        if(i_2 < (uniforms.parallelograms_count))
        {
        }
        else
        {
            break;
        }
        var _S44 : u32 = parallelograms[i_2].object_uid_0;
        var _S45 : u32 = parallelograms[i_2].material_id_0;
        var _S46 : Parallelogram_0 = Parallelogram_0( parallelograms[i_2].Q_0, parallelograms[i_2].u_0, parallelograms[i_2].object_uid_0, parallelograms[i_2].v_0, parallelograms[i_2].D_0, parallelograms[i_2].normal_0, parallelograms[i_2].w_0, parallelograms[i_2].material_id_0 );
        var _S47 : bool = hit_quad_0(_S46, 9.99999997475242708e-07f, closest_so_far_1, incident_0.ray_0);
        if(_S47)
        {
            var _S48 : vec3<f32> = hitRec.global_0.normal_1;
            hit_local_0 = hitRec.local_0;
            closest_so_far_0 = hitRec.t_2;
            hit_uid_0 = _S44;
            hit_material_id_0 = _S45;
            hit_global_normal_0 = _S48;
        }
        else
        {
            closest_so_far_0 = closest_so_far_1;
            hit_uid_0 = hit_uid_1;
            hit_material_id_0 = hit_material_id_1;
            hit_global_normal_0 = hit_global_normal_1;
        }
        var _S49 : u32 = i_2 + u32(1);
        closest_so_far_1 = closest_so_far_0;
        hit_uid_1 = hit_uid_0;
        hit_material_id_1 = hit_material_id_0;
        hit_global_normal_1 = hit_global_normal_0;
        i_2 = _S49;
    }
    var _S50 : vec3<f32> = vec3<f32>(1.0f) / incident_0.ray_0.direction_0;
    var _S51 : i32 = i32(uniforms.bvh_length);
    var node_index_0 : i32 = i32(0);
    for(;;)
    {
        var _S52 : bool;
        if(node_index_0 < _S51)
        {
            _S52 = i32(-1) != node_index_0;
        }
        else
        {
            _S52 = false;
        }
        if(_S52)
        {
        }
        else
        {
            break;
        }
        var aabb_hit_0 : AabbHit_0 = hit_aabb_0(bvh[node_index_0].aabb_min_0, bvh[node_index_0].aabb_max_0, 9.99999997475242708e-07f, closest_so_far_1, incident_0.ray_0.origin_2, _S50);
        if(aabb_hit_0.hit_0)
        {
            var _S53 : u32 = bvh[node_index_0].primitive_type_0;
            if(u32(2) == (bvh[node_index_0].primitive_type_0))
            {
                var _S54 : u32 = triangles[bvh[node_index_0].primitive_index_0].object_uid_1;
                var _S55 : u32 = triangles[bvh[node_index_0].primitive_index_0].material_id_1;
                var _S56 : Triangle_0 = Triangle_0( triangles[bvh[node_index_0].primitive_index_0].A_0, triangles[bvh[node_index_0].primitive_index_0].B_0, triangles[bvh[node_index_0].primitive_index_0].C_0, triangles[bvh[node_index_0].primitive_index_0].normalA_0, triangles[bvh[node_index_0].primitive_index_0].normalB_0, triangles[bvh[node_index_0].primitive_index_0].object_uid_1, triangles[bvh[node_index_0].primitive_index_0].normalC_0, triangles[bvh[node_index_0].primitive_index_0].material_id_1 );
                var _S57 : bool = hit_triangle_0(_S56, 9.99999997475242708e-07f, closest_so_far_1, incident_0.ray_0);
                if(_S57)
                {
                    var _S58 : vec3<f32> = hitRec.global_0.normal_1;
                    hit_local_0 = hitRec.local_0;
                    closest_so_far_0 = hitRec.t_2;
                    hit_uid_0 = _S54;
                    hit_material_id_0 = _S55;
                    hit_global_normal_0 = _S58;
                }
                else
                {
                    closest_so_far_0 = closest_so_far_1;
                    hit_uid_0 = hit_uid_1;
                    hit_material_id_0 = hit_material_id_1;
                    hit_global_normal_0 = hit_global_normal_1;
                }
            }
            else
            {
                if(u32(1) == _S53)
                {
                    var _S59 : u32 = sdf[bvh[node_index_0].primitive_index_0].material_id_2;
                    var _S60 : u32 = sdf[bvh[node_index_0].primitive_index_0].object_uid_2;
                    var _S61 : Sdf_0 = Sdf_0( sdf[bvh[node_index_0].primitive_index_0].location_col_0_0, sdf[bvh[node_index_0].primitive_index_0].location_col_1_0, sdf[bvh[node_index_0].primitive_index_0].location_col_2_0, sdf[bvh[node_index_0].primitive_index_0].inverse_location_col_0_0, sdf[bvh[node_index_0].primitive_index_0].inverse_location_col_1_0, sdf[bvh[node_index_0].primitive_index_0].inverse_location_col_2_0, sdf[bvh[node_index_0].primitive_index_0].ray_marching_step_scale_0, sdf[bvh[node_index_0].primitive_index_0].class_index_0, sdf[bvh[node_index_0].primitive_index_0].material_id_2, sdf[bvh[node_index_0].primitive_index_0].object_uid_2 );
                    var _S62 : bool = hit_sdf_0(_S61, sdf_time[bvh[node_index_0].primitive_index_0], incident_0.ray_0, aabb_hit_0.ray_parameter_0, closest_so_far_1);
                    if(_S62)
                    {
                        var _S63 : vec3<f32> = hitRec.global_0.normal_1;
                        hit_local_0 = hitRec.local_0;
                        closest_so_far_0 = hitRec.t_2;
                        hit_uid_0 = _S60;
                        hit_material_id_0 = _S59;
                        hit_global_normal_0 = _S63;
                    }
                    else
                    {
                        closest_so_far_0 = closest_so_far_1;
                        hit_uid_0 = hit_uid_1;
                        hit_material_id_0 = hit_material_id_1;
                        hit_global_normal_0 = hit_global_normal_1;
                    }
                }
                else
                {
                    closest_so_far_0 = closest_so_far_1;
                    hit_uid_0 = hit_uid_1;
                    hit_material_id_0 = hit_material_id_1;
                    hit_global_normal_0 = hit_global_normal_1;
                }
            }
            node_index_0 = node_index_0 + i32(1);
        }
        else
        {
            var _S64 : i32 = bvh[node_index_0].hit_miss_skip_link_0;
            closest_so_far_0 = closest_so_far_1;
            hit_uid_0 = hit_uid_1;
            hit_material_id_0 = hit_material_id_1;
            hit_global_normal_0 = hit_global_normal_1;
            node_index_0 = _S64;
        }
        closest_so_far_1 = closest_so_far_0;
        hit_uid_1 = hit_uid_0;
        hit_material_id_1 = hit_material_id_0;
        hit_global_normal_1 = hit_global_normal_0;
    }
    if(u32(0) < hit_uid_1)
    {
        var _S65 : Material_0 = Material_0( materials[hit_material_id_1].albedo_0, materials[hit_material_id_1].emission_0, materials[hit_material_id_1].specular_0, materials[hit_material_id_1].specular_strength_0, materials[hit_material_id_1].roughness_0, materials[hit_material_id_1].refractive_index_eta_0, materials[hit_material_id_1].albedo_texture_uid_0, materials[hit_material_id_1].material_class_0 );
        var _S66 : vec3<f32> = fetch_albedo_0(hit_local_0, incident_0.ray_0.direction_0, closest_so_far_1, _S65, incident_0.differentials_0);
        hit_global_normal_0 = _S66;
    }
    else
    {
        hit_global_normal_0 = _S43;
    }
    var result_9 : FirstHitSurface_0;
    result_9.object_uid_3 = hit_uid_1;
    result_9.albedo_1 = hit_global_normal_0;
    result_9.normal_3 = hit_global_normal_1;
    return result_9;
}

var<private> randState : u32;

@compute
@workgroup_size(8, 8, 1)
fn compute_surface_attributes_buffer(@builtin(global_invocation_id) global_invocation_id_1 : vec3<u32>)
{
    randState = u32(0);
    var pixel_index_2 : u32 = evaluate_pixel_index_0(global_invocation_id_1, uniforms.thread_grid_size);
    if(pixel_outside_frame_buffer_0(pixel_index_2))
    {
        return;
    }
    var pixel_3 : Pixel_0 = setup_pixel_coordinates_0(pixel_index_2);
    var surface_intersection_0 : FirstHitSurface_0 = trace_first_intersection_0(ray_and_differentials_0(setup_camera_0(), pixel_3, 0.5f, 0.5f));
    object_id_buffer[pixel_index_2] = surface_intersection_0.object_uid_3;
    albedo_buffer[pixel_index_2] = vec4<f32>(surface_intersection_0.albedo_1.xyz, 1.0f);
    normal_buffer[pixel_index_2] = vec4<f32>(surface_intersection_0.normal_3, 0.0f);
    return;
}

var<private> lights : Parallelogram_0;

fn get_lights_0()
{
    var i_3 : u32 = u32(0);
    for(;;)
    {
        if(i_3 < (uniforms.parallelograms_count))
        {
        }
        else
        {
            break;
        }
        if((any(((materials[parallelograms[i_3].material_id_0].emission_0.xyz) != vec3<f32>(0.0f)))))
        {
            var _S67 : vec3<f32> = parallelograms[i_3].u_0;
            var _S68 : u32 = parallelograms[i_3].object_uid_0;
            var _S69 : vec3<f32> = parallelograms[i_3].v_0;
            var _S70 : f32 = parallelograms[i_3].D_0;
            var _S71 : vec3<f32> = parallelograms[i_3].normal_0;
            var _S72 : vec3<f32> = parallelograms[i_3].w_0;
            var _S73 : u32 = parallelograms[i_3].material_id_0;
            lights.Q_0 = parallelograms[i_3].Q_0;
            lights.u_0 = _S67;
            lights.object_uid_0 = _S68;
            lights.v_0 = _S69;
            lights.D_0 = _S70;
            lights.normal_0 = _S71;
            lights.w_0 = _S72;
            lights.material_id_0 = _S73;
            break;
        }
        i_3 = i_3 + u32(1);
    }
    return;
}

fn make_common_color_evaluation_setup_0( pixel_index_3 : u32) -> Pixel_0
{
    var pixel_4 : Pixel_0 = setup_pixel_coordinates_0(pixel_index_3);
    get_lights_0();
    return pixel_4;
}

fn rand_0_1_0() -> f32
{
    var _S74 : u32 = randState * u32(747796405) + u32(2891336453);
    randState = _S74;
    var word_0 : u32 = ((((_S74 >> ((((_S74 >> (u32(28)))) + u32(4))))) ^ (_S74))) * u32(277803737);
    return f32((((word_0 >> (u32(22)))) ^ (word_0))) / 4.294967296e+09f;
}

var<private> hitMaterial : Material_0;

fn hit_scene_0( ray_6 : Ray_0,  max_ray_patameter_0 : f32) -> bool
{
    var hit_anything_0 : bool;
    var closest_so_far_2 : f32;
    var closest_so_far_3 : f32 = max_ray_patameter_0;
    var hit_anything_1 : bool = false;
    var i_4 : u32 = u32(0);
    for(;;)
    {
        if(i_4 < (uniforms.parallelograms_count))
        {
        }
        else
        {
            break;
        }
        var _S75 : Parallelogram_0 = Parallelogram_0( parallelograms[i_4].Q_0, parallelograms[i_4].u_0, parallelograms[i_4].object_uid_0, parallelograms[i_4].v_0, parallelograms[i_4].D_0, parallelograms[i_4].normal_0, parallelograms[i_4].w_0, parallelograms[i_4].material_id_0 );
        var _S76 : bool = hit_quad_0(_S75, 9.99999997475242708e-07f, closest_so_far_3, ray_6);
        if(_S76)
        {
            closest_so_far_2 = hitRec.t_2;
            hit_anything_0 = true;
        }
        else
        {
            closest_so_far_2 = closest_so_far_3;
            hit_anything_0 = hit_anything_1;
        }
        var _S77 : u32 = i_4 + u32(1);
        closest_so_far_3 = closest_so_far_2;
        hit_anything_1 = hit_anything_0;
        i_4 = _S77;
    }
    var _S78 : vec3<f32> = vec3<f32>(1.0f) / ray_6.direction_0;
    var _S79 : i32 = i32(uniforms.bvh_length);
    var node_index_1 : i32 = i32(0);
    for(;;)
    {
        var _S80 : bool;
        if(node_index_1 < _S79)
        {
            _S80 = i32(-1) != node_index_1;
        }
        else
        {
            _S80 = false;
        }
        if(_S80)
        {
        }
        else
        {
            break;
        }
        var aabb_hit_1 : AabbHit_0 = hit_aabb_0(bvh[node_index_1].aabb_min_0, bvh[node_index_1].aabb_max_0, 9.99999997475242708e-07f, closest_so_far_3, ray_6.origin_2, _S78);
        if(aabb_hit_1.hit_0)
        {
            var _S81 : u32 = bvh[node_index_1].primitive_type_0;
            if(u32(2) == (bvh[node_index_1].primitive_type_0))
            {
                var _S82 : Triangle_0 = Triangle_0( triangles[bvh[node_index_1].primitive_index_0].A_0, triangles[bvh[node_index_1].primitive_index_0].B_0, triangles[bvh[node_index_1].primitive_index_0].C_0, triangles[bvh[node_index_1].primitive_index_0].normalA_0, triangles[bvh[node_index_1].primitive_index_0].normalB_0, triangles[bvh[node_index_1].primitive_index_0].object_uid_1, triangles[bvh[node_index_1].primitive_index_0].normalC_0, triangles[bvh[node_index_1].primitive_index_0].material_id_1 );
                var _S83 : bool = hit_triangle_0(_S82, 9.99999997475242708e-07f, closest_so_far_3, ray_6);
                if(_S83)
                {
                    closest_so_far_2 = hitRec.t_2;
                    hit_anything_0 = true;
                }
                else
                {
                    closest_so_far_2 = closest_so_far_3;
                    hit_anything_0 = hit_anything_1;
                }
            }
            else
            {
                if(u32(1) == _S81)
                {
                    var _S84 : Sdf_0 = Sdf_0( sdf[bvh[node_index_1].primitive_index_0].location_col_0_0, sdf[bvh[node_index_1].primitive_index_0].location_col_1_0, sdf[bvh[node_index_1].primitive_index_0].location_col_2_0, sdf[bvh[node_index_1].primitive_index_0].inverse_location_col_0_0, sdf[bvh[node_index_1].primitive_index_0].inverse_location_col_1_0, sdf[bvh[node_index_1].primitive_index_0].inverse_location_col_2_0, sdf[bvh[node_index_1].primitive_index_0].ray_marching_step_scale_0, sdf[bvh[node_index_1].primitive_index_0].class_index_0, sdf[bvh[node_index_1].primitive_index_0].material_id_2, sdf[bvh[node_index_1].primitive_index_0].object_uid_2 );
                    var _S85 : bool = hit_sdf_0(_S84, sdf_time[bvh[node_index_1].primitive_index_0], ray_6, aabb_hit_1.ray_parameter_0, closest_so_far_3);
                    if(_S85)
                    {
                        closest_so_far_2 = hitRec.t_2;
                        hit_anything_0 = true;
                    }
                    else
                    {
                        closest_so_far_2 = closest_so_far_3;
                        hit_anything_0 = hit_anything_1;
                    }
                }
                else
                {
                    closest_so_far_2 = closest_so_far_3;
                    hit_anything_0 = hit_anything_1;
                }
            }
            node_index_1 = node_index_1 + i32(1);
        }
        else
        {
            var _S86 : i32 = bvh[node_index_1].hit_miss_skip_link_0;
            closest_so_far_2 = closest_so_far_3;
            hit_anything_0 = hit_anything_1;
            node_index_1 = _S86;
        }
        closest_so_far_3 = closest_so_far_2;
        hit_anything_1 = hit_anything_0;
    }
    var _S87 : vec3<f32> = materials[hitRec.material_id_3].emission_0;
    var _S88 : vec3<f32> = materials[hitRec.material_id_3].specular_0;
    var _S89 : f32 = materials[hitRec.material_id_3].specular_strength_0;
    var _S90 : f32 = materials[hitRec.material_id_3].roughness_0;
    var _S91 : f32 = materials[hitRec.material_id_3].refractive_index_eta_0;
    var _S92 : i32 = materials[hitRec.material_id_3].albedo_texture_uid_0;
    var _S93 : i32 = materials[hitRec.material_id_3].material_class_0;
    hitMaterial.albedo_0 = materials[hitRec.material_id_3].albedo_0;
    hitMaterial.emission_0 = _S87;
    hitMaterial.specular_0 = _S88;
    hitMaterial.specular_strength_0 = _S89;
    hitMaterial.roughness_0 = _S90;
    hitMaterial.refractive_index_eta_0 = _S91;
    hitMaterial.albedo_texture_uid_0 = _S92;
    hitMaterial.material_class_0 = _S93;
    return hit_anything_1;
}

var<private> doSpecular : f32;

var<private> unit_w : vec3<f32>;

var<private> v : vec3<f32>;

var<private> u : vec3<f32>;

fn onb_build_from_w_0( w_2 : vec3<f32>) -> mat3x3<f32>
{
    unit_w = w_2;
    var a_0 : vec3<f32>;
    if((abs(w_2.x)) > 0.89999997615814209f)
    {
        a_0 = vec3<f32>(0.0f, 1.0f, 0.0f);
    }
    else
    {
        a_0 = vec3<f32>(1.0f, 0.0f, 0.0f);
    }
    var _S94 : vec3<f32> = normalize(cross(unit_w, a_0));
    v = _S94;
    var _S95 : vec3<f32> = cross(unit_w, _S94);
    u = _S95;
    return mat3x3<f32>(_S95, v, unit_w);
}

fn cosine_sampling_wrt_Z_0() -> vec3<f32>
{
    var r1_0 : f32 = rand_0_1_0();
    var r2_0 : f32 = rand_0_1_0();
    var phi_0 : f32 = 6.28318548202514648f * r1_0;
    var _S96 : f32 = sqrt(r2_0);
    return vec3<f32>(cos(phi_0) * _S96, sin(phi_0) * _S96, sqrt(1.0f - r2_0));
}

fn onb_get_local_0( a_1 : vec3<f32>) -> vec3<f32>
{
    return u * vec3<f32>(a_1.x) + v * vec3<f32>(a_1.y) + unit_w * vec3<f32>(a_1.z);
}

struct ScatterRecord_0
{
     pdf_0 : f32,
     skip_pdf_0 : bool,
     skip_pdf_ray_0 : Ray_0,
};

var<private> scatterRec : ScatterRecord_0;

fn uniform_random_in_unit_sphere_0() -> vec3<f32>
{
    var _S97 : f32 = rand_0_1_0();
    var phi_1 : f32 = _S97 * 2.0f * 3.14159274101257324f;
    var _S98 : f32 = rand_0_1_0();
    var theta_0 : f32 = acos(2.0f * _S98 - 1.0f);
    var _S99 : f32 = sin(theta_0);
    return vec3<f32>(_S99 * cos(phi_1), _S99 * sin(phi_1), cos(theta_0));
}

fn reflectance_0( cosine_0 : f32,  ref_idx_0 : f32) -> f32
{
    var r0_0 : f32 = (1.0f - ref_idx_0) / (1.0f + ref_idx_0);
    var r0_1 : f32 = r0_0 * r0_0;
    return r0_1 + (1.0f - r0_1) * pow(1.0f - cosine_0, 5.0f);
}

fn near_zero_scalar_0( v_2 : f32) -> bool
{
    return (abs(v_2)) < 0.00009999999747379f;
}

fn near_zero_0( v_3 : vec3<f32>) -> bool
{
    var _S100 : bool;
    if(near_zero_scalar_0(v_3[i32(0)]))
    {
        _S100 = near_zero_scalar_0(v_3[i32(1)]);
    }
    else
    {
        _S100 = false;
    }
    if(_S100)
    {
        _S100 = near_zero_scalar_0(v_3[i32(2)]);
    }
    else
    {
        _S100 = false;
    }
    return _S100;
}

fn glass_scatter_0( hit_2 : HitRecord_0,  refractive_index_eta_1 : f32,  in_ray_direction_0 : vec3<f32>,  stochastic_0 : bool) -> Ray_0
{
    var ir_0 : f32;
    if(hit_2.front_face_0)
    {
        ir_0 = 1.0f / refractive_index_eta_1;
    }
    else
    {
        ir_0 = refractive_index_eta_1;
    }
    var cos_theta_0 : f32 = min(- dot(in_ray_direction_0, hit_2.global_0.normal_1), 1.0f);
    var direction_3 : vec3<f32>;
    if((ir_0 * sqrt(1.0f - cos_theta_0 * cos_theta_0)) > 1.0f)
    {
        direction_3 = reflect(in_ray_direction_0, hit_2.global_0.normal_1);
    }
    else
    {
        if(stochastic_0)
        {
            var _S101 : f32 = reflectance_0(cos_theta_0, ir_0);
            var _S102 : f32 = rand_0_1_0();
            if(_S101 > _S102)
            {
                direction_3 = reflect(in_ray_direction_0, hitRec.global_0.normal_1);
            }
            else
            {
                direction_3 = refract(in_ray_direction_0, hitRec.global_0.normal_1, ir_0);
            }
        }
        else
        {
            direction_3 = refract(in_ray_direction_0, hitRec.global_0.normal_1, ir_0);
        }
    }
    if(near_zero_0(direction_3))
    {
        direction_3 = hitRec.global_0.normal_1;
    }
    var result_10 : Ray_0;
    result_10.origin_2 = hitRec.global_0.position_0;
    result_10.direction_0 = direction_3;
    return result_10;
}

fn material_scatter_0( ray_in_0 : Ray_0) -> Ray_0
{
    var scattered_0 : Ray_0;
    var _S103 : vec3<f32> = vec3<f32>(0.0f);
    scattered_0.origin_2 = _S103;
    scattered_0.direction_0 = _S103;
    doSpecular = 0.0f;
    if(i32(0) == (hitMaterial.material_class_0))
    {
        var _S104 : mat3x3<f32> = onb_build_from_w_0(hitRec.global_0.normal_1);
        var diffuse_dir_0 : vec3<f32> = cosine_sampling_wrt_Z_0();
        var diffuse_dir_1 : vec3<f32> = normalize(onb_get_local_0(diffuse_dir_0));
        scattered_0.origin_2 = hitRec.global_0.position_0;
        scattered_0.direction_0 = diffuse_dir_1;
        var _S105 : f32 = rand_0_1_0();
        var _S106 : f32;
        if(_S105 < (hitMaterial.specular_strength_0))
        {
            _S106 = 1.0f;
        }
        else
        {
            _S106 = 0.0f;
        }
        doSpecular = _S106;
        var specular_dir_0 : vec3<f32> = normalize(mix(reflect(ray_in_0.direction_0, hitRec.global_0.normal_1), diffuse_dir_1, vec3<f32>(hitMaterial.roughness_0)));
        scattered_0.origin_2 = hitRec.global_0.position_0;
        scattered_0.direction_0 = normalize(mix(diffuse_dir_1, specular_dir_0, vec3<f32>(doSpecular)));
        scatterRec.skip_pdf_0 = false;
        if(doSpecular == 1.0f)
        {
            scatterRec.skip_pdf_0 = true;
            scatterRec.skip_pdf_ray_0 = scattered_0;
        }
    }
    else
    {
        if(i32(1) == (hitMaterial.material_class_0))
        {
            var reflected_0 : vec3<f32> = reflect(ray_in_0.direction_0, hitRec.global_0.normal_1);
            scattered_0.origin_2 = hitRec.global_0.position_0;
            var _S107 : f32 = hitMaterial.roughness_0;
            var _S108 : vec3<f32> = uniform_random_in_unit_sphere_0();
            scattered_0.direction_0 = normalize(reflected_0 + vec3<f32>(_S107) * _S108);
            scatterRec.skip_pdf_0 = true;
            scatterRec.skip_pdf_ray_0 = scattered_0;
        }
        else
        {
            if(i32(2) == (hitMaterial.material_class_0))
            {
                var _S109 : Ray_0 = glass_scatter_0(hitRec, hitMaterial.refractive_index_eta_0, ray_in_0.direction_0, true);
                scattered_0 = _S109;
                scatterRec.skip_pdf_0 = true;
                scatterRec.skip_pdf_ray_0 = scattered_0;
            }
            else
            {
                if(i32(3) == (hitMaterial.material_class_0))
                {
                    var _S110 : f32 = hitMaterial.specular_strength_0 * hitMaterial.specular_strength_0;
                    var _S111 : f32 = 1.0f + _S110;
                    var _S112 : f32 = 1.0f - _S110;
                    var _S113 : f32 = 1.0f - hitMaterial.specular_strength_0;
                    var _S114 : f32 = 2.0f * hitMaterial.specular_strength_0;
                    var _S115 : f32 = rand_0_1_0();
                    var cos_hg_0 : f32 = (_S111 - pow(_S112 / (_S113 + _S114 * _S115), 2.0f)) / _S114;
                    var sin_hg_0 : f32 = sqrt(1.0f - cos_hg_0 * cos_hg_0);
                    var _S116 : f32 = rand_0_1_0();
                    var phi_2 : f32 = 6.28318548202514648f * _S116;
                    var hg_dir_0 : vec3<f32> = vec3<f32>(sin_hg_0 * cos(phi_2), sin_hg_0 * sin(phi_2), cos_hg_0);
                    var _S117 : mat3x3<f32> = onb_build_from_w_0(ray_in_0.direction_0);
                    scattered_0.origin_2 = hitRec.global_0.position_0;
                    scattered_0.direction_0 = normalize(onb_get_local_0(hg_dir_0));
                    scatterRec.skip_pdf_0 = true;
                    scatterRec.skip_pdf_ray_0 = scattered_0;
                }
            }
        }
    }
    return scattered_0;
}

fn get_random_on_quad_0( q_0 : Parallelogram_0,  origin_4 : vec3<f32>) -> Ray_0
{
    var _S118 : f32 = rand_0_1_0();
    var _S119 : vec3<f32> = q_0.Q_0 + vec3<f32>(_S118) * q_0.u_0;
    var _S120 : f32 = rand_0_1_0();
    var p_0 : vec3<f32> = _S119 + vec3<f32>(_S120) * q_0.v_0;
    var result_11 : Ray_0;
    result_11.origin_2 = origin_4;
    result_11.direction_0 = normalize(p_0 - origin_4);
    return result_11;
}

fn onb_lambertian_scattering_pdf_0( scattered_1 : Ray_0) -> f32
{
    return max(0.0f, dot(normalize(scattered_1.direction_0), unit_w) / 3.14159274101257324f);
}

fn light_pdf_0( ray_7 : Ray_0,  quad_1 : Parallelogram_0) -> f32
{
    var _S121 : f32 = dot(ray_7.direction_0, quad_1.normal_0);
    if(_S121 > 0.0f)
    {
        return 0.00009999999747379f;
    }
    var denom_1 : f32 = dot(quad_1.normal_0, ray_7.direction_0);
    if((abs(denom_1)) < 9.99999993922529029e-09f)
    {
        return 0.00009999999747379f;
    }
    var t_4 : f32 = (quad_1.D_0 - dot(quad_1.normal_0, ray_7.origin_2)) / denom_1;
    var _S122 : bool;
    if(t_4 <= 0.00100000004749745f)
    {
        _S122 = true;
    }
    else
    {
        _S122 = t_4 >= 1.0e+09f;
    }
    if(_S122)
    {
        return 0.00009999999747379f;
    }
    var planar_hitpt_vector_1 : vec3<f32> = at_0(ray_7, t_4) - quad_1.Q_0;
    var alpha_1 : f32 = dot(quad_1.w_0, cross(planar_hitpt_vector_1, quad_1.v_0));
    var beta_1 : f32 = dot(quad_1.w_0, cross(quad_1.u_0, planar_hitpt_vector_1));
    if(alpha_1 < 0.0f)
    {
        _S122 = true;
    }
    else
    {
        _S122 = 1.0f < alpha_1;
    }
    if(_S122)
    {
        _S122 = true;
    }
    else
    {
        _S122 = beta_1 < 0.0f;
    }
    if(_S122)
    {
        _S122 = true;
    }
    else
    {
        _S122 = 1.0f < beta_1;
    }
    if(_S122)
    {
        return 0.00009999999747379f;
    }
    var hitNormal_0 : vec3<f32>;
    if((_S121 < 0.0f) == false)
    {
        hitNormal_0 = (vec3<f32>(0) - quad_1.normal_0);
    }
    else
    {
        hitNormal_0 = quad_1.normal_0;
    }
    var _S123 : f32 = length(ray_7.direction_0);
    return t_4 * t_4 * _S123 * _S123 / (abs(dot(ray_7.direction_0, hitNormal_0) / _S123) * length(cross(lights.u_0, lights.v_0)));
}

fn ray_color_monte_carlo_0( incident_1 : RayAndDifferentials_0) -> vec3<f32>
{
    var current_ray_0 : Ray_0 = incident_1.ray_0;
    var _S124 : vec3<f32> = vec3<f32>(0.0f);
    var _S125 : vec3<f32> = vec3<f32>(1.0f);
    var i_5 : i32 = i32(0);
    var throughput_0 : vec3<f32> = _S125;
    var accumulated_radiance_0 : vec3<f32> = _S124;
    for(;;)
    {
        if(i_5 < i32(50))
        {
        }
        else
        {
            break;
        }
        var _S126 : bool = hit_scene_0(current_ray_0, 1.0e+09f);
        if(_S126 == false)
        {
            accumulated_radiance_0 = accumulated_radiance_0 + vec3<f32>(0.10000000149011612f) * throughput_0;
            break;
        }
        var albedo_color_0 : vec3<f32> = fetch_albedo_0(hitRec.local_0, current_ray_0.direction_0, hitRec.t_2, hitMaterial, incident_1.differentials_0);
        var _S127 : vec3<f32> = hitMaterial.emission_0.xyz;
        var emission_color_0 : vec3<f32>;
        if(!hitRec.front_face_0)
        {
            emission_color_0 = _S124;
        }
        else
        {
            emission_color_0 = _S127;
        }
        var scatterred_surface_0 : Ray_0 = material_scatter_0(current_ray_0);
        if(scatterRec.skip_pdf_0)
        {
            var accumulated_radiance_1 : vec3<f32> = accumulated_radiance_0 + emission_color_0 * throughput_0;
            var throughput_1 : vec3<f32> = throughput_0 * mix(albedo_color_0, hitMaterial.specular_0, vec3<f32>(doSpecular));
            current_ray_0 = scatterRec.skip_pdf_ray_0;
            current_ray_0.origin_2 = current_ray_0.origin_2 + current_ray_0.direction_0 * vec3<f32>(0.00050000002374873f);
            throughput_0 = throughput_1;
            accumulated_radiance_0 = accumulated_radiance_1;
            i_5 = i_5 + i32(1);
            continue;
        }
        var _S128 : f32 = rand_0_1_0();
        var scattered_2 : Ray_0;
        if(_S128 > 0.20000000298023224f)
        {
            scattered_2 = scatterred_surface_0;
        }
        else
        {
            var _S129 : Ray_0 = get_random_on_quad_0(lights, hitRec.global_0.position_0);
            scattered_2 = _S129;
        }
        var lambertian_pdf_0 : f32 = onb_lambertian_scattering_pdf_0(scattered_2);
        var pdf_1 : f32 = 0.20000000298023224f * light_pdf_0(scattered_2, lights) + 0.80000001192092896f * lambertian_pdf_0;
        if(pdf_1 <= 0.00000999999974738f)
        {
            return emission_color_0 * throughput_0;
        }
        var accumulated_radiance_2 : vec3<f32> = accumulated_radiance_0 + emission_color_0 * throughput_0;
        var throughput_2 : vec3<f32> = throughput_0 * (vec3<f32>(lambertian_pdf_0) * mix(albedo_color_0, hitMaterial.specular_0, vec3<f32>(doSpecular)) / vec3<f32>(pdf_1));
        current_ray_0 = scattered_2;
        current_ray_0.origin_2 = current_ray_0.origin_2 + current_ray_0.direction_0 * vec3<f32>(0.00050000002374873f);
        var throughput_3 : vec3<f32>;
        if(i_5 > i32(2))
        {
            var p_1 : f32 = max(throughput_2.x, max(throughput_2.y, throughput_2.z));
            var _S130 : f32 = rand_0_1_0();
            if(_S130 > p_1)
            {
                accumulated_radiance_0 = accumulated_radiance_2;
                break;
            }
            throughput_3 = throughput_2 * vec3<f32>((1.0f / p_1));
        }
        else
        {
            throughput_3 = throughput_2;
        }
        throughput_0 = throughput_3;
        accumulated_radiance_0 = accumulated_radiance_2;
        i_5 = i_5 + i32(1);
    }
    return accumulated_radiance_0;
}

fn path_trace_monte_carlo_0( camera_4 : Camera_0,  pixel_5 : Pixel_0) -> vec3<f32>
{
    var samples_count_0 : u32 = uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision;
    var _S131 : vec3<f32> = vec3<f32>(0.0f);
    var i_6 : u32 = u32(0);
    var result_color_0 : vec3<f32> = _S131;
    for(;;)
    {
        if(i_6 < samples_count_0)
        {
        }
        else
        {
            break;
        }
        var sub_pixel_x_3 : f32 = rand_0_1_0();
        var sub_pixel_y_3 : f32 = rand_0_1_0();
        var _S132 : vec3<f32> = ray_color_monte_carlo_0(ray_and_differentials_0(camera_4, pixel_5, sub_pixel_x_3, sub_pixel_y_3));
        var result_color_1 : vec3<f32> = result_color_0 + _S132;
        i_6 = i_6 + u32(1);
        result_color_0 = result_color_1;
    }
    return result_color_0 / vec3<f32>(f32(samples_count_0));
}

@compute
@workgroup_size(8, 8, 1)
fn compute_color_buffer_monte_carlo(@builtin(global_invocation_id) global_invocation_id_2 : vec3<u32>)
{
    randState = u32(0);
    var pixel_index_4 : u32 = evaluate_pixel_index_0(global_invocation_id_2, uniforms.thread_grid_size);
    if(pixel_outside_frame_buffer_0(pixel_index_4))
    {
        return;
    }
    var camera_5 : Camera_0 = setup_camera_0();
    var pixel_6 : Pixel_0 = make_common_color_evaluation_setup_0(pixel_index_4);
    randState = pixel_index_4 + u32(uniforms.frame_number) * u32(719393);
    var traced_color_0 : vec3<f32> = path_trace_monte_carlo_0(camera_5, pixel_6);
    pixel_color_buffer[pixel_index_4] = vec4<f32>(pixel_color_buffer[pixel_index_4].xyz + traced_color_0, 1.0f);
    return;
}

struct VSOutput_0
{
    @builtin(position) position_1 : vec4<f32>,
};

@vertex
fn vs(@builtin(vertex_index) in_vertex_index_0 : u32) -> VSOutput_0
{
    randState = u32(0);
    var output_0 : VSOutput_0;
    output_0.position_1 = vec4<f32>(full_screen_quad_positions_0[in_vertex_index_0], 0.0f, 1.0f);
    return output_0;
}

fn pixel_global_index_0( pixel_position_0 : vec2<f32>) -> u32
{
    return u32(pixel_position_0.y) * uniforms.frame_buffer_size.x + u32(pixel_position_0.x);
}

fn aces_approx_0( v_4 : vec3<f32>) -> vec3<f32>
{
    var v1_0 : vec3<f32> = v_4 * vec3<f32>(0.60000002384185791f);
    return clamp(v1_0 * (vec3<f32>(2.50999999046325684f) * v1_0 + vec3<f32>(0.02999999932944775f)) / (v1_0 * (vec3<f32>(2.43000006675720215f) * v1_0 + vec3<f32>(0.5899999737739563f)) + vec3<f32>(0.14000000059604645f)), vec3<f32>(0.0f), vec3<f32>(1.0f));
}

fn gradient_noise_0( uv_0 : vec2<f32>) -> f32
{
    return fract(52.98291778564453125f * fract(dot(uv_0, vec2<f32>(0.06711056083440781f, 0.00583714991807938f))));
}

fn pseudo_dither_0( color_0 : vec3<f32>,  pixel_coordinate_0 : vec2<f32>) -> vec3<f32>
{
    return color_0 + vec3<f32>((0.00392156885936856f * gradient_noise_0(pixel_coordinate_0))) - vec3<f32>(0.00196078442968428f);
}

struct pixelOutput_0
{
    @location(0) output_1 : vec4<f32>,
};

@fragment
fn fs(@builtin(position) position_2 : vec4<f32>) -> pixelOutput_0
{
    randState = u32(0);
    var _S133 : vec2<f32> = position_2.xy;
    var _S134 : pixelOutput_0 = pixelOutput_0( vec4<f32>(pseudo_dither_0(pow(aces_approx_0((pixel_color_buffer[pixel_global_index_0(_S133)].xyz / vec3<f32>(uniforms.frame_number)).xyz).xyz, vec3<f32>(0.45454543828964233f)), _S133), 1.0f) );
    return _S134;
}

fn evaluate_hard_shadow_0( position_3 : vec3<f32>,  to_light_0 : vec3<f32>,  min_ray_offset_0 : f32,  max_ray_offset_0 : f32) -> f32
{
    var _S135 : bool = hit_scene_0(Ray_x24init_0(position_3 + to_light_0 * vec3<f32>(min_ray_offset_0), to_light_0), max_ray_offset_0);
    if(_S135)
    {
        if((any(((hitMaterial.emission_0.xyz) > vec3<f32>(0.0f, 0.0f, 0.0f)))))
        {
            return 1.0f;
        }
        return 0.0f;
    }
    return 1.0f;
}

fn inside_aabb_0( box_min_1 : vec3<f32>,  box_max_1 : vec3<f32>,  probe_0 : vec3<f32>) -> bool
{
    var _S136 : bool;
    if((all((probe_0 >= box_min_1))))
    {
        _S136 = (all((probe_0 <= box_max_1)));
    }
    else
    {
        _S136 = false;
    }
    return _S136;
}

fn sample_signed_distance_function_0( sdf_4 : Sdf_0,  position_4 : vec3<f32>,  direction_4 : vec3<f32>,  time_4 : f32) -> f32
{
    var sdf_inverse_location_1 : mat3x4<f32> = mat3x4<f32>(sdf_4.inverse_location_col_0_0, sdf_4.inverse_location_col_1_0, sdf_4.inverse_location_col_2_0);
    var local_position_1 : vec3<f32> = transform_point_0(sdf_inverse_location_1, position_4);
    var local_direction_0 : vec3<f32> = normalize(transform_vector_0(to_mat3x3_0(sdf_inverse_location_1), direction_4));
    var local_distance_0 : f32 = sample_sdf_0(sdf_4, local_position_1, time_4);
    var global_offset_0 : vec3<f32> = transform_point_0(mat3x4<f32>(sdf_4.location_col_0_0, sdf_4.location_col_1_0, sdf_4.location_col_2_0), local_position_1 + local_direction_0 * vec3<f32>(local_distance_0)) - position_4;
    return length(global_offset_0) * f32((i32(sign((dot(global_offset_0, direction_4))))));
}

fn sample_signed_distance_0( position_5 : vec3<f32>,  direction_5 : vec3<f32>) -> f32
{
    var _S137 : i32 = i32(uniforms.bvh_length);
    var record_0 : f32 = 1.0e+09f;
    var node_index_2 : i32 = i32(0);
    for(;;)
    {
        var _S138 : bool;
        if(node_index_2 < _S137)
        {
            _S138 = i32(-1) != node_index_2;
        }
        else
        {
            _S138 = false;
        }
        if(_S138)
        {
        }
        else
        {
            break;
        }
        var record_1 : f32;
        if(inside_aabb_0(bvh_inflated[node_index_2].aabb_min_0, bvh_inflated[node_index_2].aabb_max_0, position_5))
        {
            if(u32(1) == (bvh_inflated[node_index_2].primitive_type_0))
            {
                var _S139 : Sdf_0 = Sdf_0( sdf[bvh_inflated[node_index_2].primitive_index_0].location_col_0_0, sdf[bvh_inflated[node_index_2].primitive_index_0].location_col_1_0, sdf[bvh_inflated[node_index_2].primitive_index_0].location_col_2_0, sdf[bvh_inflated[node_index_2].primitive_index_0].inverse_location_col_0_0, sdf[bvh_inflated[node_index_2].primitive_index_0].inverse_location_col_1_0, sdf[bvh_inflated[node_index_2].primitive_index_0].inverse_location_col_2_0, sdf[bvh_inflated[node_index_2].primitive_index_0].ray_marching_step_scale_0, sdf[bvh_inflated[node_index_2].primitive_index_0].class_index_0, sdf[bvh_inflated[node_index_2].primitive_index_0].material_id_2, sdf[bvh_inflated[node_index_2].primitive_index_0].object_uid_2 );
                var candidate_distance_0 : f32 = sample_signed_distance_function_0(_S139, position_5, direction_5, sdf_time[bvh_inflated[node_index_2].primitive_index_0]);
                if(candidate_distance_0 < record_0)
                {
                    record_1 = candidate_distance_0;
                }
                else
                {
                    record_1 = record_0;
                }
            }
            else
            {
                record_1 = record_0;
            }
            node_index_2 = node_index_2 + i32(1);
        }
        else
        {
            var _S140 : i32 = bvh_inflated[node_index_2].hit_miss_skip_link_0;
            record_1 = record_0;
            node_index_2 = _S140;
        }
        record_0 = record_1;
    }
    return record_0;
}

fn approximate_ambient_occlusion_0( posision_0 : vec3<f32>,  normal_4 : vec3<f32>) -> f32
{
    var i_7 : i32 = i32(0);
    var fall_off_0 : f32 = 1.0f;
    var occlusion_0 : f32 = 0.0f;
    for(;;)
    {
        if(i_7 < i32(5))
        {
        }
        else
        {
            break;
        }
        var height_2 : f32 = 0.00999999977648258f + 0.11999999731779099f * f32(i_7) / 4.0f;
        var signed_distance_1 : f32 = sample_signed_distance_0(posision_0 + vec3<f32>(height_2) * normal_4, normal_4);
        var occlusion_1 : f32 = occlusion_0 + max(0.0f, (height_2 - signed_distance_1) * fall_off_0);
        var fall_off_1 : f32 = fall_off_0 * 0.94999998807907104f;
        if(occlusion_1 > 0.34999999403953552f)
        {
            occlusion_0 = occlusion_1;
            break;
        }
        i_7 = i_7 + i32(1);
        fall_off_0 = fall_off_1;
        occlusion_0 = occlusion_1;
    }
    return clamp(2.5f - 7.0f * occlusion_0, 0.0f, 1.0f);
}

fn evaluate_dielectric_surface_color_0( camera_origin_0 : vec3<f32>,  hit_3 : HitRecord_0,  hit_material_0 : Material_0,  hit_albedo_0 : vec3<f32>) -> vec3<f32>
{
    var to_light_1 : vec3<f32> = lights.Q_0 + (lights.u_0 + lights.v_0) * vec3<f32>(0.5f) - hit_3.global_0.position_0;
    var to_light_distance_0 : f32 = length(to_light_1);
    var to_light_direction_0 : vec3<f32>;
    if(to_light_distance_0 > 0.00009999999747379f)
    {
        to_light_direction_0 = to_light_1 / vec3<f32>(to_light_distance_0);
    }
    else
    {
        to_light_direction_0 = vec3<f32>(0.0f, 0.0f, 0.0f);
    }
    var diffuse_fall_off_0 : f32 = max(0.0f, dot(hit_3.global_0.normal_1, to_light_direction_0));
    var specular_fall_off_0 : f32 = pow(max(0.0f, dot(reflect((vec3<f32>(0) - to_light_direction_0), hit_3.global_0.normal_1), normalize(camera_origin_0 - hit_3.global_0.position_0))), 4.0f) * diffuse_fall_off_0;
    var shadow_0 : f32 = evaluate_hard_shadow_0(hit_3.global_0.position_0, to_light_direction_0, 0.00499999988824129f, to_light_distance_0);
    var shadow_lightened_0 : f32 = shadow_0 * 0.39999997615814209f + 0.60000002384185791f;
    var occlusion_2 : f32 = approximate_ambient_occlusion_0(hit_3.global_0.position_0, hit_3.global_0.normal_1);
    var _S141 : vec3<f32> = vec3<f32>(occlusion_2);
    return mix(vec3<f32>(diffuse_fall_off_0) * hit_albedo_0 * _S141, vec3<f32>(specular_fall_off_0) * hit_material_0.specular_0, vec3<f32>(hit_material_0.specular_strength_0)) * materials[lights.material_id_0].emission_0.xyz * vec3<f32>(shadow_lightened_0) + vec3<f32>(0.10000000149011612f) * hit_albedo_0 * _S141 + hit_material_0.emission_0.xyz;
}

fn rand_from_seed_0( seed_0 : f32) -> f32
{
    return fract(sin(seed_0) * 43758.546875f);
}

fn reflection_roughness_addition_0( position_6 : vec3<f32>,  extra_seed_0 : f32) -> vec3<f32>
{
    var _S142 : f32 = rand_0_1_0();
    var phi_3 : f32 = rand_from_seed_0((_S142 + position_6.x + 0.35699999332427979f) * extra_seed_0) * 2.0f * 3.14159274101257324f;
    var _S143 : f32 = rand_0_1_0();
    var theta_1 : f32 = acos(2.0f * rand_from_seed_0((_S143 + position_6.y + 16.35647010803222656f) * extra_seed_0) - 1.0f);
    var _S144 : f32 = sin(theta_1);
    return vec3<f32>(_S144 * cos(phi_3), _S144 * sin(phi_3), cos(theta_1));
}

fn evaluate_reflection_0( incident_2 : vec3<f32>,  normal_5 : vec3<f32>,  hit_position_0 : vec3<f32>,  roughness_1 : f32) -> vec3<f32>
{
    var perfect_0 : vec3<f32> = reflect(incident_2, normal_5);
    if(near_zero_scalar_0(roughness_1))
    {
        return perfect_0;
    }
    var _S145 : vec3<f32> = reflection_roughness_addition_0(hit_position_0, incident_2.z);
    return normalize(perfect_0 + _S145 * vec3<f32>(roughness_1));
}

fn ray_color_deterministic_0( camera_origin_1 : vec3<f32>,  incident_3 : RayAndDifferentials_0) -> vec3<f32>
{
    var accumulated_radiance_3 : vec3<f32>;
    var _S146 : vec3<f32> = vec3<f32>(0.0f);
    var current_ray_1 : Ray_0 = incident_3.ray_0;
    var _S147 : vec3<f32> = vec3<f32>(1.0f);
    var i_8 : i32 = i32(0);
    var throughput_4 : vec3<f32> = _S147;
    for(;;)
    {
        if(i_8 < i32(8))
        {
        }
        else
        {
            accumulated_radiance_3 = _S146;
            break;
        }
        var _S148 : bool = hit_scene_0(current_ray_1, 1.0e+09f);
        if(false == _S148)
        {
            accumulated_radiance_3 = vec3<f32>(0.10000000149011612f) * throughput_4;
            break;
        }
        var hit_material_1 : Material_0 = hitMaterial;
        var hit_albedo_1 : vec3<f32> = fetch_albedo_0(hitRec.local_0, current_ray_1.direction_0, hitRec.t_2, hitMaterial, incident_3.differentials_0);
        if(i32(0) == (hit_material_1.material_class_0))
        {
            var _S149 : vec3<f32> = evaluate_dielectric_surface_color_0(camera_origin_1, hitRec, hit_material_1, hit_albedo_1);
            accumulated_radiance_3 = throughput_4 * _S149;
            break;
        }
        if(i32(1) == (hit_material_1.material_class_0))
        {
            var reflected_1 : vec3<f32> = evaluate_reflection_0(current_ray_1.direction_0, hitRec.global_0.normal_1, hitRec.global_0.position_0, hit_material_1.roughness_0);
            current_ray_1.origin_2 = hitRec.global_0.position_0 + reflected_1 * vec3<f32>(0.00050000002374873f);
            current_ray_1.direction_0 = reflected_1;
            throughput_4 = throughput_4 * hit_albedo_1;
        }
        else
        {
            var _S150 : vec3<f32>;
            if(i32(2) == (hit_material_1.material_class_0))
            {
                var _S151 : Ray_0 = glass_scatter_0(hitRec, hit_material_1.refractive_index_eta_0, current_ray_1.direction_0, false);
                current_ray_1 = _S151;
                current_ray_1.origin_2 = current_ray_1.origin_2 + current_ray_1.direction_0 * vec3<f32>(0.00050000002374873f);
                _S150 = throughput_4 * hit_albedo_1;
            }
            else
            {
                accumulated_radiance_3 = hit_albedo_1;
                break;
            }
            throughput_4 = _S150;
        }
        i_8 = i_8 + i32(1);
    }
    return accumulated_radiance_3;
}

fn path_trace_deterministic_0( camera_6 : Camera_0,  pixel_7 : Pixel_0) -> vec3<f32>
{
    var _S152 : u32 = uniforms.pixel_side_subdivision;
    if((uniforms.pixel_side_subdivision) == u32(1))
    {
        var _S153 : vec3<f32> = ray_color_deterministic_0(camera_6.origin_0, ray_and_differentials_0(camera_6, pixel_7, 0.5f, 0.5f));
        return _S153;
    }
    var _S154 : vec3<f32> = vec3<f32>(0.0f);
    var _S155 : f32 = 1.0f / f32(_S152 - u32(1));
    var i_9 : u32 = u32(0);
    var result_color_2 : vec3<f32> = _S154;
    for(;;)
    {
        if(i_9 < _S152)
        {
        }
        else
        {
            break;
        }
        var j_0 : u32 = u32(0);
        for(;;)
        {
            if(j_0 < _S152)
            {
            }
            else
            {
                break;
            }
            var _S156 : vec3<f32> = ray_color_deterministic_0(camera_6.origin_0, ray_and_differentials_0(camera_6, pixel_7, _S155 * f32(i_9), _S155 * f32(j_0)));
            var result_color_3 : vec3<f32> = result_color_2 + _S156;
            j_0 = j_0 + u32(1);
            result_color_2 = result_color_3;
        }
        i_9 = i_9 + u32(1);
    }
    return result_color_2 / vec3<f32>(f32(_S152 * _S152));
}

@compute
@workgroup_size(8, 8, 1)
fn compute_color_buffer_deterministic(@builtin(global_invocation_id) global_invocation_id_3 : vec3<u32>)
{
    randState = u32(0);
    var pixel_index_5 : u32 = evaluate_pixel_index_0(global_invocation_id_3, uniforms.thread_grid_size);
    if(pixel_outside_frame_buffer_0(pixel_index_5))
    {
        return;
    }
    var camera_7 : Camera_0 = setup_camera_0();
    var pixel_8 : Pixel_0 = make_common_color_evaluation_setup_0(pixel_index_5);
    var traced_color_1 : vec3<f32> = path_trace_deterministic_0(camera_7, pixel_8);
    pixel_color_buffer[pixel_index_5] = vec4<f32>(traced_color_1, 1.0f);
    return;
}

