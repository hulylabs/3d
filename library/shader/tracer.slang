#language slang 2026

// >>> BEGIN: DUMMY STUBS
func procedural_texture_select(index: int32_t, position: float3, normal: float3, time: float, dp_dx: float3, dp_dy: float3) -> float3 {
    __intrinsic_asm "procedural_texture_select";
}

func sdf_select(index: int32_t, position: float3, time: float) -> float {
    __intrinsic_asm "sdf_select";
}

func sdf_apply_animation(index: int32_t, position: float3, time: float) -> float3 {
    __intrinsic_asm "sdf_apply_animation";
}
// >>> END: DUMMY STUBS

static const float PI = 3.1415926535897932385;

static const float MIN_FLOAT = 0.0001;
static const float MAX_FLOAT = 999999999.999;

static const float SECONDARY_RAY_START_BIAS = 0.0005;
static const float RAY_PARAMETER_MIN = 0.000001;

static const int MATERIAL_LAMBERTIAN = 0;
static const int MATERIAL_MIRROR = 1;
static const int MATERIAL_GLASS = 2;
static const int MATERIAL_ISOTROPIC = 3;

static const int TEXTURE_WRAP_MODE_REPEAT = 0;
static const int TEXTURE_WRAP_MODE_CLAMP = 1;
// static const int TEXTURE_WRAP_MODE_DISCARD = 2; - default behavior if the mode is none of the above

static const uint PRIMITIVE_TYPE_SDF = 1;
static const uint PRIMITIVE_TYPE_TRIANGLE = 2;

static const int NULL_POINTER_LINK = -1;

static const uint WORK_GROUP_SIZE_X = 8;
static const uint WORK_GROUP_SIZE_Y = 8;
static const uint WORK_GROUP_SIZE_Z = 1;
static const uint3 WORK_GROUP_SIZE = uint3(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, WORK_GROUP_SIZE_Z);

static const float3 BACKGROUND_COLOR = float3(0.1);

static const int DETERMINISTIC_AMBIENT_OCCLUSION_SAMPLES = 5;
static const int DETERMINISTIC_SHADOW_RAY_MAX_STEPS = 32;
static const float DETERMINISTIC_SHADOW_MIN_STEP = 0.01;
static const float DETERMINISTIC_SHADOW_MAX_STEP = 0.1;
static const float DETERMINISTIC_SHADOW_START_BIAS = 0.005;
static const float DETERMINISTIC_SHADOW_LIGHT_SIZE_SCALE = 8.0; // bigger - crispier shadows
static const float DETERMINISTIC_SHADOW_MARCHING_MIN = -1.0;
static const float DETERMINISTIC_SHADOW_MARCHING_MAX = 1.0;
static const float DETERMINISTIC_SHADOW_FLOOR = 0.6;
static const int DETERMINISTIC_MAX_RAY_BOUNCES = 8;

static const int MONTE_CARLO_MAX_RAY_BOUNCES = 50;
static const bool MONTE_CARLO_STRATIFY_SAMLING = false;
static const bool MONTE_CARLO_IMPORTANCE_SAMPLING = true;
static const int MAX_SDF_RAY_MARCH_STEPS = 120;

[vk::binding(0, 0)]
cbuffer uniforms {
    uint2 frame_buffer_size;
    uint frame_buffer_area;
    float frame_buffer_aspect; // width / height
    
    float2 inverted_frame_buffer_size;
    float frame_number;
    float empty_slot__1;
    
    float4 view_matrix_col_0;
    float4 view_matrix_col_1;
    float4 view_matrix_col_2;
    float4 view_matrix_col_3;
    /* Consider a view ray defined by an origin (e.g., the eye position for a perspective camera)
    and a direction that intersects the view plane at a world-space pixel position.
    This matrix, when multiplied by the world-space pixel position, returns the ray's origin.
    For a perspective camera, the origin is always the eye position â€" the same for all pixels.
    For an orthographic camera, the origin lies on the camera plane and varies per pixel. */
    float4 view_ray_origin_matrix_col_0;
    float4 view_ray_origin_matrix_col_1;
    float4 view_ray_origin_matrix_col_2;
    float4 view_ray_origin_matrix_col_3;

    uint parallelograms_count;
    uint bvh_length;
    uint pixel_side_subdivision; // anti-aliasing level: bigger value -> slower render -> less jagged edges
    float global_time_seconds;

    uint3 thread_grid_size;
    float empty_slot__2;
};

[vk::binding(1, 0)]
SamplerState atlases_sampler;
[vk::binding(2, 0)]
Texture2D<float4> texture_atlas_page;

[vk::binding(0, 1)]
RWStructuredBuffer<float4> pixel_color_buffer;
[vk::binding(1, 1)]
RWStructuredBuffer<uint> object_id_buffer;
[vk::binding(2, 1)]
RWStructuredBuffer<float4> normal_buffer;
[vk::binding(3, 1)]
RWStructuredBuffer<float4> albedo_buffer;

[vk::binding(0, 2)]
StructuredBuffer<Parallelogram> parallelograms;
[vk::binding(1, 2)]
StructuredBuffer<Sdf> sdf;
[vk::binding(2, 2)]
StructuredBuffer<Triangle> triangles;
[vk::binding(3, 2)]
StructuredBuffer<Material> materials;
[vk::binding(4, 2)]
StructuredBuffer<BvhNode> bvh;
[vk::binding(5, 2)]
StructuredBuffer<BvhNode> bvh_inflated;
[vk::binding(6, 2)]
StructuredBuffer<float> sdf_time;
[vk::binding(7, 2)]
StructuredBuffer<AtlasMapping> texture_atlases_mapping;

static uint randState = 0u;

static HitRecord hitRec;
static Material hitMaterial;
static ScatterRecord scatterRec;
static Parallelogram lights;

struct Ray {
    float3 origin;
    float3 direction;
};

struct Material {
    float3 albedo;
    float3 emission;
    float3 specular;
    float specular_strength; // chance that a ray hitting would reflect specularly
    float roughness; // diffuse strength
    float refractive_index_eta; // refractive index
    int albedo_texture_uid; // > 0 - atlas r_e_g_i_o_n index (1-based), < 0 - procedural texture uid, = 0 - none
    int material_class;
};

struct AtlasMapping {
    float2 top_left_corner_uv;
    float2 size;
    float4 local_position_to_texture_x;
    float4 local_position_to_texture_y;
    int2 wrap_mode;
};

struct Parallelogram {
    float3 Q;
    float3 u;
    uint object_uid;
    float3 v;
    float D;
    float3 normal;
    float3 w;
    uint material_id;
};

struct Triangle {
    float3 A;
    float3 B;
    float3 C;
    float3 normalA;
    float3 normalB;
    uint object_uid;
    float3 normalC;
    uint material_id;
};

struct Sdf {
    float4 location_col_0;
    float4 location_col_1;
    float4 location_col_2;
    float4 inverse_location_col_0;
    float4 inverse_location_col_1;
    float4 inverse_location_col_2;
    float ray_marching_step_scale;
    int class_index;
    uint material_id;
    uint object_uid;
};

struct BvhNode {
    float3 aabb_min;
    uint primitive_index;

    float3 aabb_max;
    uint primitive_type;

    int hit_miss_skip_link;
};

struct HitPlace {
    float3 position;
    float3 normal;
};

struct HitRecord {
    HitPlace global;
    HitPlace local;
    float t;
    uint material_id;
    bool front_face;
};

struct FirstHitSurface {
    uint object_uid;
    float3 albedo;
    float3 normal;
};

struct ScatterRecord {
    float pdf;
    bool skip_pdf;
    Ray skip_pdf_ray;
};

float3 at(Ray ray, float t) {
    return ray.origin + t * ray.direction;
}

// PCG prng
// https://www.shadertoy.com/view/XlGcRh
float rand_0_1() {
    randState = randState * 747796405u + 2891336453u;
    uint word = ((randState >> ((randState >> 28u) + 4u)) ^ randState) * 277803737u;
    return float((word >> 22u)^word) / 4294967295.0;
}

float random_double(float min_val, float max_val) {
    return min_val + (max_val - min_val) * rand_0_1();
}

bool near_zero(float3 v) {
    return near_zero_scalar(v[0]) && near_zero_scalar(v[1]) && near_zero_scalar(v[2]);
}

bool near_zero_scalar(float v) {
    return abs(v) < MIN_FLOAT;
}

float sample_sdf(Sdf sdf, float3 point, float time) {
    return sdf_select(sdf.class_index, point, time);
}

float3 apply_animation(Sdf sdf, float3 point, float time) {
    return sdf_apply_animation(sdf.class_index, point, time);
}

float3 signed_distance_normal(Sdf sdf, float3 point, float time) {
    float2 e = float2(1.0,-1.0)*0.5773*0.0005;
    return normalize( e.xyy * sample_sdf( sdf, point + e.xyy, time ) +
                      e.yyx * sample_sdf( sdf, point + e.yyx, time ) +
                      e.yxy * sample_sdf( sdf, point + e.yxy, time ) +
                      e.xxx * sample_sdf( sdf, point + e.xxx, time ) );
}

float3 transform_point(float3x4 transformation, float3 point) {
    return mul(transformation, float4(point, 1.0f));
}

float3x3 to_mat3x3(float3x4 source) {
    return float3x3(source[0].xyz, source[1].xyz, source[2].xyz);
}

float3 transform_vector(float3x3 transformation, float3 vector) {
    return mul(transformation, vector);
}

float3 transform_transposed_vector(float3x3 transformation, float3 vector) {
    /*Sdf matrices come from CPU in row-major format, so
    we need to multiply like v * M. Swaping operands like
    M * v equals to v * transpose(M).*/
    return mul(transpose(transformation), vector);
}

float transform_ray_parameter(float3x4 transformation, Ray ray, float parameter, float3 transformed_origin) {
    float3 point = transform_point(transformation, at(ray, parameter));
    return length(point - transformed_origin);
}

bool hit_sdf(Sdf sdf, float time, Ray ray, float tmin, float tmax) {
    float3x4 sdf_inverse_location = float3x4(sdf.inverse_location_col_0,sdf.inverse_location_col_1,sdf.inverse_location_col_2);
    float3x3 sdf_location_inverse = to_mat3x3(sdf_inverse_location);
    float3 local_ray_origin = transform_point(sdf_inverse_location, ray.origin);
    float3 local_ray_direction = normalize(transform_vector(sdf_location_inverse, ray.direction));
    Ray local_ray = Ray(local_ray_origin, local_ray_direction);

    float local_t = transform_ray_parameter(sdf_inverse_location, ray, tmin, local_ray_origin);
    float local_t_max = transform_ray_parameter(sdf_inverse_location, ray, tmax, local_ray_origin);

    int i = 0;
    while (true) {
        if (i >= MAX_SDF_RAY_MARCH_STEPS) {
            break;
        }
        if (local_t>local_t_max) {
            break;
        }

        float3 candidate = at(local_ray, local_t);
        float signed_distance = sample_sdf(sdf, candidate, time);
        float t_scaled = 0.0001 * local_t;

        if(abs(signed_distance) < t_scaled) {
            hitRec.local.normal = signed_distance_normal(sdf, candidate, time);
            hitRec.global.normal = normalize(transform_transposed_vector(sdf_location_inverse, hitRec.local.normal));

            float3x4 sdf_location = float3x4(sdf.location_col_0,sdf.location_col_1,sdf.location_col_2);
            hitRec.global.position = transform_point(sdf_location, candidate);
            hitRec.local.position = apply_animation(sdf, candidate, time);

            hitRec.t = length(hitRec.global.position - ray.origin);

            hitRec.front_face = sample_sdf(sdf, local_ray.origin, time) >= 0;
            if(hitRec.front_face == false) {
                hitRec.global.normal = -hitRec.global.normal;
                hitRec.local.normal = -hitRec.local.normal;
            }

            hitRec.material_id = sdf.material_id;
            return true;
        }

        float step_size = max(abs(signed_distance) * sdf.ray_marching_step_scale, t_scaled);
        local_t += step_size;
        i = i + 1;
    }

    return false;
}

bool hit_quad(Parallelogram quad, float tmin, float tmax, Ray ray) {

    if(dot(ray.direction, quad.normal) > 0) {
        return false;
    }

    float denom = dot(quad.normal, ray.direction);

    // no hit if the ray is paraller to the plane
    if(abs(denom) < 1e-8) {
        return false;
    }

    float t = (quad.D - dot(quad.normal, ray.origin)) / denom;
    if(t <= tmin || t >= tmax) {
        return false;
    }

    // determine if hit point lies within quarilateral
    float3 intersection = at(ray, t);
    float3 planar_hitpt_vector = intersection - quad.Q;
    float alpha = dot(quad.w, cross(planar_hitpt_vector, quad.v));
    float beta = dot(quad.w, cross(quad.u, planar_hitpt_vector));

    if(alpha < 0 || 1 < alpha || beta < 0 || 1 < beta) {
        return false;
    }

    hitRec.t = t;

    float3 local_position = quad.u * alpha + quad.v * beta;
    /* To match coordinate frame of the SDFs (all of them
    centered in kind of mass center) - so texture coordinates
    of thin cube matches with parallelogram. */
    hitRec.local.position = local_position - (quad.u + quad.v) * 0.5;
    hitRec.global.position = quad.Q + local_position;

    hitRec.global.normal = quad.normal;
    hitRec.front_face = denom < 0.0;
    if(false == hitRec.front_face) {
        hitRec.global.normal = -hitRec.global.normal;
    }
    hitRec.local.normal = hitRec.global.normal;

    hitRec.material_id = quad.material_id;

    return true;
}

// https://stackoverflow.com/questions/42740765/
// https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection.html
bool hit_triangle(Triangle triangle, float tmin, float tmax, Ray ray) {
    float3 AB = triangle.B - triangle.A;
    float3 AC = triangle.C - triangle.A;
    float3 normal = cross(AB, AC);
    float determinant = -dot(ray.direction, normal);

    // CULLING
    if(abs(determinant) < tmin) {
        return false;
    }

    float3 ao = ray.origin - triangle.A;
    float3 dao = cross(ao, ray.direction);

    // calculate dist to triangle & barycentric coordinates of intersection point
    float invDet = 1.0 / determinant;
    float dst = dot(ao, normal) * invDet;
    float u = dot(AC, dao) * invDet;
    float v = -dot(AB, dao) * invDet;
    float w = 1 - u - v;

    if(dst < tmin || dst > tmax || u < tmin || v < tmin || w < tmin) {
        return false;
    }

    hitRec.t = dst;
    hitRec.global.position = triangle.A * w + triangle.B * u + triangle.C * v;
    hitRec.local.position = hitRec.global.position;

    hitRec.global.normal = normalize(triangle.normalA * w + triangle.normalB * u + triangle.normalC * v);
    hitRec.front_face = dot(ray.direction, hitRec.global.normal) < 0;
    if(hitRec.front_face == false) {
        hitRec.global.normal = -hitRec.global.normal;
    }

    hitRec.local.normal = hitRec.global.normal;
    hitRec.material_id = triangle.material_id;

    return true;
}

struct AabbHit {
    bool hit;
    float ray_parameter;
};

// https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
AabbHit hit_aabb(float3 box_min, float3 box_max, float tmin, float tmax, float3 ray_origin, float3 inverted_ray_dir) {
    float3 t0s = (box_min - ray_origin) * inverted_ray_dir;
    float3 t1s = (box_max - ray_origin) * inverted_ray_dir;

    float3 tsmaller = min(t0s, t1s);
    float3 tbigger = max(t0s, t1s);

    float t_min = max(tmin, max(tsmaller.x, max(tsmaller.y, tsmaller.z)));
    float t_max = min(tmax, min(tbigger.x, min(tbigger.y, tbigger.z)));

    AabbHit result;
    result.hit = t_max > t_min;
    result.ray_parameter = t_min;
    return result;
}

void get_lights() {
    for(uint i = 0u; i < uniforms.parallelograms_count; i++) {
        float3 emission = materials[parallelograms[i].material_id].emission.rgb;

        if(any(emission != float3(0.0))) {
            lights = parallelograms[i];
            break;
        }
    }
}

uint evaluate_pixel_index(uint3 global_invocation_id, uint3 thread_grid_size) {
    return
        global_invocation_id.z * (thread_grid_size.x * thread_grid_size.y) +
        global_invocation_id.y * (thread_grid_size.x) +
        global_invocation_id.x ;
}

struct Pixel {
    float2 coordinates;
};

Pixel setup_pixel_coordinates(uint pixel_index) {
    uint x = pixel_index % uniforms.frame_buffer_size.x;
    uint y = pixel_index / uniforms.frame_buffer_size.x;
    Pixel result;
    result.coordinates = float2(float(x), float(y));
    return result;
}

Camera setup_camera() {
    float fov_factor = 1.0 / tan(60 * (PI / 180.0) / 2.0);
    float3 origin = uniforms.view_matrix_col_3.xyz;
    Camera result;
    result.fov_factor = fov_factor;
    result.origin = origin;
    return result;
}

bool pixel_outside_frame_buffer(uint pixel_index) {
    return pixel_index >= uniforms.frame_buffer_area;
}

[shader("compute")]
[numthreads(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1)]
void compute_surface_attributes_buffer(uint3 global_invocation_id : SV_DispatchThreadID) {
    uint pixel_index = evaluate_pixel_index(global_invocation_id, uniforms.thread_grid_size);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

    Pixel pixel = setup_pixel_coordinates(pixel_index);
    Camera camera = setup_camera();

    RayAndDifferentials ray = ray_and_differentials(camera, pixel, 0.5, 0.5);
    FirstHitSurface surface_intersection = trace_first_intersection(ray);
    object_id_buffer[pixel_index] = surface_intersection.object_uid;
    albedo_buffer[pixel_index] = float4(surface_intersection.albedo.rgb, 1.0f);
    normal_buffer[pixel_index] = float4(surface_intersection.normal, 0.0f);
}

Pixel make_common_color_evaluation_setup(uint pixel_index) {
    Pixel pixel = setup_pixel_coordinates(pixel_index);
    get_lights();
    return pixel;
}

[shader("compute")]
[numthreads(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1)]
void compute_color_buffer_monte_carlo(uint3 global_invocation_id : SV_DispatchThreadID) {
    uint pixel_index = evaluate_pixel_index(global_invocation_id, uniforms.thread_grid_size);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

    Camera camera = setup_camera();
    Pixel pixel = make_common_color_evaluation_setup(pixel_index);

    randState = pixel_index + uint(uniforms.frame_number) * 719393;
    float3 traced_color = path_trace_monte_carlo(camera, pixel);

    pixel_color_buffer[pixel_index] = float4(pixel_color_buffer[pixel_index].xyz + traced_color, 1.0);
}

struct Camera {
    float fov_factor;
    float3 origin;
};

/*
x = aspect * (2 * (x / width) - 1)     [ranges from -aspect to +aspect]
y = -(2 * (y / height) - 1)            [ranges from +1 to -1]
lower left pixel corner -> 0.5, 0.5 gives pixel's center;
lower left pixel corner -> 0.5, 0.5 gives pixel's center;
*/
Ray ray_to_pixel(Camera camera, Pixel pixel, float sub_pixel_x, float sub_pixel_y) {
    float s = uniforms.frame_buffer_aspect * (2 * ((pixel.coordinates.x + sub_pixel_x) * uniforms.inverted_frame_buffer_size.x) - 1);
    float t = -1 * (2 * ((pixel.coordinates.y + sub_pixel_y) * uniforms.inverted_frame_buffer_size.y) - 1);
    return get_camera_ray(camera, s, t);
}

Ray get_camera_ray(Camera camera, float s, float t) {
    float4x4 view_matrix = float4x4(uniforms.view_matrix_col_0, uniforms.view_matrix_col_1, uniforms.view_matrix_col_2, uniforms.view_matrix_col_3);
    float3 eye_to_pixel_direction = mul(float4(float3(s, t, -camera.fov_factor), 0.0), view_matrix).xyz;
    float4 pixel_world_space = float4(camera.origin + eye_to_pixel_direction, 1.0);

    float4x4 view_ray_origin_matrix = float4x4(uniforms.view_ray_origin_matrix_col_0, uniforms.view_ray_origin_matrix_col_1, uniforms.view_ray_origin_matrix_col_2, uniforms.view_ray_origin_matrix_col_3);
    float3 ray_origin_world_space = mul(pixel_world_space, view_ray_origin_matrix).xyz;
    float3 direction = normalize(pixel_world_space.xyz - ray_origin_world_space);

    Ray result;
    result.origin = ray_origin_world_space;
    result.direction = direction;
    return result;
}

struct RayDifferentials {
    float3 dx;
    float3 dy;
};

RayDifferentials ray_differentials(Camera camera, Pixel pixel, float sub_pixel_x, float sub_pixel_y) {
    Pixel pixel_dx;
    pixel_dx.coordinates = pixel.coordinates + float2(1.0, 0.0);
    Ray ray_direction_dx = ray_to_pixel(camera, pixel_dx, sub_pixel_x, sub_pixel_y);
    
    Pixel pixel_dy;
    pixel_dy.coordinates = pixel.coordinates + float2(0.0, 1.0);
    Ray ray_direction_dy = ray_to_pixel(camera, pixel_dy, sub_pixel_x, sub_pixel_y);
    
    RayDifferentials result;
    result.dx = ray_direction_dx.direction;
    result.dy = ray_direction_dy.direction;
    return result;
}

struct RayAndDifferentials {
    Ray ray;
    RayDifferentials differentials;
};

RayAndDifferentials ray_and_differentials(Camera camera, Pixel pixel, float sub_pixel_x, float sub_pixel_y) {
    Ray ray = ray_to_pixel(camera, pixel, sub_pixel_x, sub_pixel_y);
    RayDifferentials differentials = ray_differentials(camera, pixel, sub_pixel_x, sub_pixel_y);
    
    RayAndDifferentials result;
    result.ray = ray;
    result.differentials = differentials;
    return result;
}

// position partial derivatives (with respect to x and y axes)
struct RayDerivatives {
    float3 dp_dx;
    float3 dp_dy;
};

/*
We have a ray (O, dÌ‚) that intersects a surface at point P. There is also a second ray (O, dÌ‚â‚").
The surface at point P has a normal N â€" weâ€™ll assume the surface is a plane (with that same
normal N and passing through P). For smooth surfaces, in a sufficiently small neighborhood
of P, this is actually not far from reality.

Letâ€™s denote the intersection of the second ray with this plane as Pâ‚". What we need is the
vector (Pâ‚" - P) â€" this will be an approximation of the footprint that the pixel (through which
the original ray passed) has on the surface.

P = O + t * dÌ‚ where t is a scalar.
Pâ‚" = O + tâ‚" * dÌ‚â‚" where tâ‚" is some other scalar (which we donâ€™t know).

All points lying on our plane satisfy the equation (?, N) - (P, N) = 0.
Letâ€™s plug Pâ‚" and P into this:

(O + tâ‚" * dÌ‚â‚" - P, N) = 0
(O, N) + tâ‚" * (dÌ‚â‚", N) - (P, N) = 0
(O, N) + tâ‚" * (dÌ‚â‚", N) - (O + t * dÌ‚, N) = 0
(O, N) + tâ‚" * (dÌ‚â‚", N) - (O, N) - t * (dÌ‚, N) = 0
tâ‚" * (dÌ‚â‚", N) - t * (dÌ‚, N) = 0
tâ‚" = t * (dÌ‚, N) / (dÌ‚â‚", N)

Now:

Pâ‚" - P =
O + tâ‚" * dÌ‚â‚" - O - t * dÌ‚ =
tâ‚" * dÌ‚â‚" - t * dÌ‚ =
t * ((dÌ‚, N) / (dÌ‚â‚", N) * dÌ‚â‚" - dÌ‚)

So, Pâ‚" - P = t * ((dÌ‚, N) / (dÌ‚â‚", N) * dÌ‚â‚" - dÌ‚)
*/

RayDerivatives ray_hit_position_derivatives(
    float3 ray_direction,
    float surface_intersection_parameter,
    float3 surface_normal,
    RayDifferentials ray_differentials
) {
    float ray_dot_normal = dot(ray_direction, surface_normal);
    float3 dp_dx = surface_intersection_parameter * (ray_differentials.dx * ray_dot_normal / dot(ray_differentials.dx, surface_normal) - ray_direction);
    float3 dp_dy = surface_intersection_parameter * (ray_differentials.dy * ray_dot_normal / dot(ray_differentials.dy, surface_normal) - ray_direction);
    
    RayDerivatives result;
    result.dp_dx = dp_dx;
    result.dp_dy = dp_dy;
    return result;
}

float3 path_trace_monte_carlo(Camera camera, Pixel pixel) {
    uint samples_count = uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision;
    float3 result_color = float3(0.0);
    if(MONTE_CARLO_STRATIFY_SAMLING) {
        float reciprocal_sqrt_samples_per_pixel = 1.0 / float(uniforms.pixel_side_subdivision);
        for(uint i = 0u; i < uniforms.pixel_side_subdivision; i++) {
            for(uint j = 0u; j < uniforms.pixel_side_subdivision; j++) {
                float sub_pixel_x = reciprocal_sqrt_samples_per_pixel * (float(i) + rand_0_1());
                float sub_pixel_y = reciprocal_sqrt_samples_per_pixel * (float(j) + rand_0_1());
                RayAndDifferentials ray = ray_and_differentials(camera, pixel, sub_pixel_x, sub_pixel_y);
                result_color += ray_color_monte_carlo(ray);
            }
        }
    } else {
        for(uint i = 0u; i < samples_count; i++) {
            float sub_pixel_x = rand_0_1();
            float sub_pixel_y = rand_0_1();
            RayAndDifferentials ray = ray_and_differentials(camera, pixel, sub_pixel_x, sub_pixel_y);
            result_color += ray_color_monte_carlo(ray);
        }
    }
    result_color /= float(samples_count);

    return result_color;
}

FirstHitSurface trace_first_intersection(RayAndDifferentials incident) {
    Ray ray = incident.ray;

    float closest_so_far = MAX_FLOAT;
    uint hit_uid = 0;
    uint hit_material_id = 0;
    float3 hit_global_normal = float3(0.0f);
    HitPlace hit_local;
    hit_local.position = float3(0.0f);
    hit_local.normal = float3(0.0f);

    for(uint i = 0u; i < uniforms.parallelograms_count; i++){
        Parallelogram parallelogram = parallelograms[i];
        if(hit_quad(parallelogram, RAY_PARAMETER_MIN, closest_so_far, ray)) {
            hit_uid = parallelogram.object_uid;
            hit_material_id = parallelogram.material_id;
            hit_global_normal = hitRec.global.normal;
            hit_local = hitRec.local;
            closest_so_far = hitRec.t;
        }
    }

    // "Implementing a practical rendering system using GLSL" by Toshiya Hachisuka
    {
        float3 inverted_ray_dir = 1.0 / ray.direction;
        int node_index = 0;
        int max_index = int(uniforms.bvh_length);
        while (node_index < max_index && NULL_POINTER_LINK != node_index) {
            BvhNode node = bvh[node_index];
            AabbHit aabb_hit = hit_aabb(node.aabb_min, node.aabb_max, RAY_PARAMETER_MIN, closest_so_far, ray.origin, inverted_ray_dir);
            if(aabb_hit.hit) {
                if(PRIMITIVE_TYPE_TRIANGLE == node.primitive_type) {
                    Triangle triangle = triangles[node.primitive_index];
                    if(hit_triangle(triangle, RAY_PARAMETER_MIN, closest_so_far, ray)) {
                        hit_uid = triangle.object_uid;
                        hit_material_id = triangle.material_id;
                        hit_global_normal = hitRec.global.normal;
                        hit_local = hitRec.local;
                        closest_so_far = hitRec.t;
                    }
                } else if (PRIMITIVE_TYPE_SDF == node.primitive_type) {
                    Sdf sdf_obj = sdf[node.primitive_index];
                    if(hit_sdf(sdf_obj, sdf_time[node.primitive_index], ray, aabb_hit.ray_parameter, closest_so_far)) {
                        hit_uid = sdf_obj.object_uid;
                        hit_material_id = sdf_obj.material_id;
                        hit_global_normal = hitRec.global.normal;
                        hit_local = hitRec.local;
                        closest_so_far = hitRec.t;
                    }
                }
                node_index++;
            } else {
                node_index = node.hit_miss_skip_link;
            }
        }
    }

    float3 hit_albedo;
    if (0 < hit_uid) {
        hit_albedo = fetch_albedo(hit_local, incident.ray.direction, closest_so_far, materials[hit_material_id], incident.differentials);
    } else {
        hit_albedo = float3(0.0);
    }

    FirstHitSurface result;
    result.object_uid = hit_uid;
    result.albedo = hit_albedo;
    result.normal = hit_global_normal;
    return result;
}

float3 snap_to_grid(float3 victim, float grid_step) {
    return floor((victim - grid_step * sign(victim)) / float3(grid_step)) * grid_step;
}

float3 fetch_albedo(HitPlace hit, float3 ray_direction, float ray_parameter, Material material, RayDifferentials differentials) {
    float3 result = material.albedo.rgb;
    if (material.albedo_texture_uid < 0) {
        /*
        Grid snapping reduces visual flickering caused by floating-point precision issues
        during ray-surface intersection. The problem is especially pronounced when using
        discontinuous procedural textures (e.g., checkerboard patterns), where tiny
        differences in hit positionsâ€"particularly along cube edges and cornersâ€"can cause
        abrupt texture color changes.

        By snapping the hit position to a small 3D grid (via quantization), we ensure
        that nearby intersection points consistently evaluate to the same texture value,
        reducing instability in rendered outpu
        */
        static const float grid_step = 1e-4;
        float3 snapped_position = snap_to_grid(hit.position, grid_step);
        RayDerivatives derivartives = ray_hit_position_derivatives(ray_direction, ray_parameter, hit.normal, differentials);

        result *= procedural_texture_select(
            -material.albedo_texture_uid,
            snapped_position, hit.normal,
            uniforms.global_time_seconds,
            derivartives.dp_dx,
            derivartives.dp_dy
        );
    } else if (material.albedo_texture_uid > 0) {
        int region_index = material.albedo_texture_uid - 1;
        AtlasMapping atlas_region_mapping = texture_atlases_mapping[region_index];
        RayDerivatives derivartives = ray_hit_position_derivatives(ray_direction, ray_parameter, hit.normal, differentials);

        float4 texture_sample = read_atlas(hit.position, atlas_region_mapping, derivartives);
        result = (1.0 - texture_sample.a) * result + texture_sample.a * texture_sample.rgb;
    }
    return result;
}

float4 read_atlas(float3 local_space_position, AtlasMapping atlas_region_mapping, RayDerivatives differentials) {
    float2x4 local_position_to_texture = float2x4(atlas_region_mapping.local_position_to_texture_x, atlas_region_mapping.local_position_to_texture_y);
    float2 texture_coordinate = mul(local_position_to_texture, float4(local_space_position, 1.0));
    float2 ddx = mul(local_position_to_texture, float4(differentials.dp_dx, 0.0));
    float2 ddy = mul(local_position_to_texture, float4(differentials.dp_dy, 0.0));

    for (int i = 0; i < 2; i++) {
        float coordinate = texture_coordinate[i];
        int mode = atlas_region_mapping.wrap_mode[i];
        float inset = pixel_half_size(texture_atlas_page, ddx, ddy)[i];
        float min_edge = inset/atlas_region_mapping.size[i];
        float max_edge = 1.0 - inset/atlas_region_mapping.size[i];
        if (TEXTURE_WRAP_MODE_REPEAT == mode) {
            texture_coordinate[i] = frac(coordinate);
        } else if (TEXTURE_WRAP_MODE_CLAMP == mode) {
            texture_coordinate[i] = clamp(coordinate, min_edge, max_edge);
        } else {
            if (coordinate < min_edge || coordinate > max_edge) {
                return float4(0.0);
            }
        }
    }

    float2 uv = atlas_region_mapping.top_left_corner_uv + texture_coordinate.xy * atlas_region_mapping.size;
    float4 texture_sample = texture_atlas_page.SampleGrad(atlases_sampler, uv, ddx.xy, ddy.xy);

    return texture_sample;
}

uint calculate_mip_level(Texture2D<float4> target_texture, float2 ddx, float2 ddy) {
    uint width, height, mip_levels;
    target_texture.GetDimensions(0, width, height, mip_levels);
    float2 texture_size = float2(width, height);

    // uv gradients -> texel gradients
    float2 ddx_texel = ddx * texture_size;
    float2 ddy_texel = ddy * texture_size;

    // maximum change per pixel
    float delta_max_sqr = max(length(ddx_texel), length(ddy_texel));

    if (delta_max_sqr <= 0.0) {
        return 0;
    }

    // calculate mip level using the same formula as GPU hardware
    uint mip_level = uint(0.5 * log2(delta_max_sqr));

    return clamp(mip_level, 0, mip_levels - 1);
}

float2 pixel_half_size(Texture2D<float4> target_texture, float2 ddx, float2 ddy) {
   uint mip_level = calculate_mip_level(target_texture, ddx, ddy);
   uint width, height, mip_level_count;
   target_texture.GetDimensions(mip_level, width, height, mip_level_count);
   float2 mip_size = float2(width, height);
   return 0.5 / mip_size;
}

bool hit_scene(Ray ray, float max_ray_patameter) {
    float closest_so_far = max_ray_patameter;
    bool hit_anything = false;

    for(uint i = 0u; i < uniforms.parallelograms_count; i++) {
        if(hit_quad(parallelograms[i], RAY_PARAMETER_MIN, closest_so_far, ray)) {
            hit_anything = true;
            closest_so_far = hitRec.t;
        }
    }

    // BVH traversal from "Implementing a practical rendering system using GLSL" by Toshiya Hachisuka
    {
        float3 inverted_ray_dir = 1.0 / ray.direction;
        int node_index = 0;
        int max_index = int(uniforms.bvh_length);
        while (node_index < max_index && NULL_POINTER_LINK != node_index) {
            BvhNode node = bvh[node_index];
            AabbHit aabb_hit = hit_aabb(node.aabb_min, node.aabb_max, RAY_PARAMETER_MIN, closest_so_far, ray.origin, inverted_ray_dir);
            if(aabb_hit.hit) {
                if(PRIMITIVE_TYPE_TRIANGLE == node.primitive_type) {
                    if(hit_triangle(triangles[node.primitive_index], RAY_PARAMETER_MIN, closest_so_far, ray)) {
                        hit_anything = true;
                        closest_so_far = hitRec.t;
                    }
                } else if (PRIMITIVE_TYPE_SDF == node.primitive_type) {
                    if(hit_sdf(sdf[node.primitive_index], sdf_time[node.primitive_index], ray, aabb_hit.ray_parameter, closest_so_far)) {
                        hit_anything = true;
                        closest_so_far = hitRec.t;
                    }
                }
                node_index++;
            } else {
                node_index = node.hit_miss_skip_link;
            }
        }
    }

    hitMaterial = materials[hitRec.material_id];
    return hit_anything;
}

// https://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Path_Tracing#Implementation

float3 ray_color_monte_carlo(RayAndDifferentials incident) {

    Ray current_ray = incident.ray;
    float3 accumulated_radiance = float3(0.0);
    float3 throughput = float3(1.0);

    for(int i = 0; i < MONTE_CARLO_MAX_RAY_BOUNCES; i++) {
        if(hit_scene(current_ray, MAX_FLOAT) == false) {
            accumulated_radiance += BACKGROUND_COLOR * throughput;
            break;
        }

        // TODO: differentials should be recalculated for each bounce; using same data for rays other than first ray (from eye) is incorrect
        float3 albedo_color = fetch_albedo(hitRec.local, current_ray.direction, hitRec.t, hitMaterial, incident.differentials);
        float3 emission_color = hitMaterial.emission.rgb;
        if(!hitRec.front_face) {
            emission_color = float3(0.0);
        }

        if(MONTE_CARLO_IMPORTANCE_SAMPLING) {
            Ray scatterred_surface = material_scatter(current_ray);

            if(scatterRec.skip_pdf) {
                accumulated_radiance += emission_color * throughput;
                throughput *= lerp(albedo_color, hitMaterial.specular, doSpecular);

                current_ray = scatterRec.skip_pdf_ray;
                current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
                continue;
            }

            static const float LIGHT_SAMPLING_PROBABILITY = 0.2;
            Ray scattered;
            if(rand_0_1() > LIGHT_SAMPLING_PROBABILITY) {
                scattered = scatterred_surface;
            } else {
                scattered = get_random_on_quad(lights, hitRec.global.position);
            }

            float lambertian_pdf = onb_lambertian_scattering_pdf(scattered);
            float light_pdf_val = light_pdf(scattered, lights);
            float pdf = LIGHT_SAMPLING_PROBABILITY * light_pdf_val + (1.0 - LIGHT_SAMPLING_PROBABILITY) * lambertian_pdf;

            if(pdf <= 0.00001) {
                return emission_color * throughput;
            }

            accumulated_radiance += emission_color * throughput;
            throughput *= ((lambertian_pdf * lerp(albedo_color, hitMaterial.specular, doSpecular)) / pdf);
            current_ray = scattered;
            current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
        } else {
            Ray scattered = material_scatter(current_ray);

            accumulated_radiance += emission_color * throughput;
            throughput *= lerp(albedo_color, hitMaterial.specular, doSpecular);

            current_ray = scattered;
            current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
        }

        // russian roulette
        if(i > 2) {
            float p = max(throughput.x, max(throughput.y, throughput.z));
            if(rand_0_1() > p) {
                break;
            }

            throughput *= (1.0 / p);
        }
    }

    return accumulated_radiance;
}

static float doSpecular;
Ray material_scatter(Ray ray_in) {
    Ray scattered;
    scattered.origin = float3(0.0);
    scattered.direction = float3(0.0);
    doSpecular = 0;
    if(MATERIAL_LAMBERTIAN == hitMaterial.material_class) {

        float3x3 uvw = onb_build_from_w(hitRec.global.normal);
        float3 diffuse_dir = cosine_sampling_wrt_Z();
        diffuse_dir = normalize(onb_get_local(diffuse_dir));

        scattered.origin = hitRec.global.position;
        scattered.direction = diffuse_dir;

        doSpecular = rand_0_1() < hitMaterial.specular_strength ? 1.0 : 0.0;

        float3 specular_dir = reflect(ray_in.direction, hitRec.global.normal);
        specular_dir = normalize(lerp(specular_dir, diffuse_dir, hitMaterial.roughness));

        scattered.origin = hitRec.global.position;
        scattered.direction = normalize(lerp(diffuse_dir, specular_dir, doSpecular));

        scatterRec.skip_pdf = false;

        if(doSpecular == 1.0) {
            scatterRec.skip_pdf = true;
            scatterRec.skip_pdf_ray = scattered;
        }
    }
    else if(MATERIAL_MIRROR == hitMaterial.material_class) {
        float3 reflected = reflect(ray_in.direction, hitRec.global.normal);
        scattered.origin = hitRec.global.position;
        scattered.direction = normalize(reflected + hitMaterial.roughness * uniform_random_in_unit_sphere());

        scatterRec.skip_pdf = true;
        scatterRec.skip_pdf_ray = scattered;
    }
    else if(MATERIAL_GLASS == hitMaterial.material_class) {
        bool stochastic = true;
        scattered = glass_scatter(hitRec, hitMaterial.refractive_index_eta, ray_in.direction, stochastic);

        scatterRec.skip_pdf = true;
        scatterRec.skip_pdf_ray = scattered;
    }
    else if(MATERIAL_ISOTROPIC == hitMaterial.material_class) {
        float g = hitMaterial.specular_strength;
        float cos_hg = (1 + g*g - pow(((1 - g*g) / (1 - g + 2*g*rand_0_1())), 2.0)) / (2 * g);
        float sin_hg = sqrt(1 - cos_hg * cos_hg);
        float phi = 2 * PI * rand_0_1();

        float3 hg_dir = float3(sin_hg * cos(phi), sin_hg * sin(phi), cos_hg);

        float3x3 uvw = onb_build_from_w(ray_in.direction);
        scattered.origin = hitRec.global.position;
        scattered.direction = normalize(onb_get_local(hg_dir));

        scatterRec.skip_pdf = true;
        scatterRec.skip_pdf_ray = scattered;
    }

    return scattered;
}

Ray glass_scatter(HitRecord hit, float refractive_index_eta, float3 in_ray_direction, bool stochastic) {
    float ir = refractive_index_eta;
    if(hit.front_face) {
        ir = (1.0 / ir);
    }

    float3 unit_direction = in_ray_direction;
    float cos_theta = min(-dot(unit_direction, hit.global.normal), 1.0);
    float sin_theta = sqrt(1 - cos_theta*cos_theta);

    float3 direction = float3(0.0);
    if(ir * sin_theta > 1.0) {
        direction = reflect(unit_direction, hit.global.normal);
    } else {
        if (stochastic) {
            if (reflectance(cos_theta, ir) > rand_0_1()) {
                direction = reflect(unit_direction, hitRec.global.normal);
            } else {
                direction = refract(unit_direction, hitRec.global.normal, ir);
            }
        } else {
            direction = refract(unit_direction, hitRec.global.normal, ir);
        }
    }

    if(near_zero(direction)) {
        direction = hitRec.global.normal;
    }

    Ray result;
    result.origin = hitRec.global.position;
    result.direction = direction;
    return result;
}

float reflectance(float cosine, float ref_idx) {
    float r0 = (1 - ref_idx) / (1 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1 - r0) * pow((1 - cosine), 5.0);
}

float3 uniform_random_in_unit_sphere() {
    float phi = rand_0_1() * 2.0 * PI;
    float theta = acos(2.0 * rand_0_1() - 1.0);

    float x = sin(theta) * cos(phi);
    float y = sin(theta) * sin(phi);
    float z = cos(theta);

    return float3(x, y, z);
}

float3 random_in_unit_disk() {
    float theta = 2 * PI * rand_0_1();
    return float3(cos(theta), sin(theta), 0);
}

float3 uniform_sampling_hemisphere() {
    float3 on_unit_sphere = uniform_random_in_unit_sphere();
    float sign_dot = dot(on_unit_sphere, hitRec.global.normal) > 0.0 ? 0.0 : 1.0;
    return normalize(lerp(on_unit_sphere, -on_unit_sphere, sign_dot));
}

float3 cosine_sampling_hemisphere() {
    return uniform_random_in_unit_sphere() + hitRec.global.normal;
}

// generates a random direction weighted by PDF = cos_theta / PI relative to z axis
float3 cosine_sampling_wrt_Z() {
    float r1 = rand_0_1();
    float r2 = rand_0_1();

    float phi = 2 * PI * r1;
    float x = cos(phi) * sqrt(r2);
    float y = sin(phi) * sqrt(r2);
    float z = sqrt(1 - r2);

    return float3(x, y, z);
}

float lambertian_scattering_pdf(Ray scattered) {
    float cos_theta = max(0.0, dot(hitRec.global.normal, scattered.direction));
    return cos_theta / PI;
}

float uniform_scattering_pdf(Ray scattered) {
    return 1 / (2 * PI);
}

static float3 unit_w;
static float3 u;
static float3 v;
// creates an orthonormal basis
float3x3 onb_build_from_w(float3 w) {
    unit_w = w;
    float3 a = abs(unit_w.x) > 0.9 ? float3(0, 1, 0) : float3(1, 0, 0);
    v = normalize(cross(unit_w, a));
    u = cross(unit_w, v);

    return float3x3(u, v, unit_w);
}

float3 onb_get_local(float3 a) {
    return u * a.x + v * a.y + unit_w * a.z;
}

float onb_lambertian_scattering_pdf(Ray scattered) {
    float cosine_theta = dot(normalize(scattered.direction), unit_w);
    return max(0.0, cosine_theta/PI);
}

Ray get_random_on_quad(Parallelogram q, float3 origin) {
    float3 p = q.Q + (rand_0_1() * q.u) + (rand_0_1() * q.v);
    Ray result;
    result.origin = origin;
    result.direction = normalize(p - origin);
    return result;
}

float3 get_random_on_quad_point(Parallelogram q) {
    float3 p = q.Q + (rand_0_1() * q.u) + (rand_0_1() * q.v);
    return p;
}

float light_pdf(Ray ray, Parallelogram quad) {

    if(dot(ray.direction, quad.normal) > 0) {
        return MIN_FLOAT;
    }

    float denom = dot(quad.normal, ray.direction);

    if(abs(denom) < 1e-8) {
        return MIN_FLOAT;
    }

    float t = (quad.D - dot(quad.normal, ray.origin)) / denom;
    if(t <= 0.001 || t >= MAX_FLOAT) {
        return MIN_FLOAT;
    }

    float3 intersection = at(ray, t);
    float3 planar_hitpt_vector = intersection - quad.Q;
    float alpha = dot(quad.w, cross(planar_hitpt_vector, quad.v));
    float beta = dot(quad.w, cross(quad.u, planar_hitpt_vector));

    if(alpha < 0 || 1 < alpha || beta < 0 || 1 < beta) {
        return MIN_FLOAT;
    }

    float3 hitNormal = quad.normal;
    bool front_face = dot(ray.direction, quad.normal) < 0;
    if(front_face == false)
    {
        hitNormal = -hitNormal;
    }

    float distance_squared = t * t * length(ray.direction) * length(ray.direction);
    float cosine = abs(dot(ray.direction, hitNormal) / length(ray.direction));

    return (distance_squared / (cosine * length(cross(lights.u, lights.v))));
}

//===================================================================
// final image output (aka resolve): tone mapping + gamma correction
//===================================================================

static const float2 full_screen_quad_positions[6] = {
    float2(-1.0, -1.0),
    float2 (1.0, -1.0),
    float2(-1.0,  1.0),

    float2(-1.0,  1.0),
    float2( 1.0, -1.0),
    float2( 1.0,  1.0),
};

// ACES approximation for tone mapping
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/:
float3 aces_approx(float3 v) {
    float3 v1 = v * 0.6f; // TODO: WTF?
    static const float a = 2.51f;
    static const float b = 0.03f;
    static const float c = 2.43f;
    static const float d = 0.59f;
    static const float e = 0.14f;
    return clamp((v1*(a*v1+b))/(v1*(c*v1+d)+e), float3(0.0f), float3(1.0f));
}

struct VSOutput {
    float4 position : SV_Position;
};

[shader("vertex")]
VSOutput vs(uint in_vertex_index : SV_VertexID) {
    VSOutput output;
    output.position = float4(full_screen_quad_positions[in_vertex_index], 0.0, 1.0);
    return output;
}

uint pixel_global_index(float2 pixel_position) {
    return uint(pixel_position.y) * uniforms.frame_buffer_size.x + uint(pixel_position.x);
}

/*
Gradient noise from Jorge Jimenez's presentation:
Next Generation Post Processing in Call of Duty: Advanced Warfare
http://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare

Good post on usage: https://blog.frost.kiwi/GLSL-noise-and-radial-gradient/
*/
float gradient_noise(float2 uv) {
    return frac(52.9829189 * frac(dot(uv, float2(0.06711056, 0.00583715))));
}

float3 pseudo_dither(float3 color, float2 pixel_coordinate) {
    return color + (1.0 / 255.0) * gradient_noise(pixel_coordinate) - (0.5 / 255.0);
}

[shader("fragment")]
float4 fs(VSOutput input) : SV_Target0 {
    uint i = pixel_global_index(input.position.xy);
    float3 color = pixel_color_buffer[i].xyz / uniforms.frame_number;

    color = aces_approx(color.xyz);
    color = pow(color.xyz, float3(1.0 / 2.2));
    color = pseudo_dither(color, input.position.xy);

    return float4(color, 1.0);
}

//===================================================================
// deterministic ray tracing
//===================================================================

[shader("compute")]
[numthreads(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1)]
void compute_color_buffer_deterministic(uint3 global_invocation_id : SV_DispatchThreadID) {
    uint pixel_index = evaluate_pixel_index(global_invocation_id, uniforms.thread_grid_size);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

    Camera camera = setup_camera();
    Pixel pixel = make_common_color_evaluation_setup(pixel_index);

    float3 traced_color = path_trace_deterministic(camera, pixel);
    pixel_color_buffer[pixel_index] = float4(traced_color, 1.0);
}

float3 path_trace_deterministic(Camera camera, Pixel pixel) {
    if (uniforms.pixel_side_subdivision == 1) {
        return ray_color_deterministic(camera.origin, ray_and_differentials(camera, pixel, 0.5, 0.5));
    }

    float3 result_color = float3(0.0);
    float sub_pixel_step = 1.0 / float(uniforms.pixel_side_subdivision - 1);
    for(uint i = 0u; i < uniforms.pixel_side_subdivision; i++) {
        for(uint j = 0u; j < uniforms.pixel_side_subdivision; j++) {
            RayAndDifferentials ray = ray_and_differentials(camera, pixel, sub_pixel_step * float(i), sub_pixel_step * float(j));
            result_color += ray_color_deterministic(camera.origin, ray);
        }
    }
    result_color /= float(uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision);

    return result_color;
}

float3 ray_color_deterministic(float3 camera_origin, RayAndDifferentials incident) {
    float3 accumulated_radiance = float3(0.0);

    Ray current_ray = incident.ray;
    float3 throughput = float3(1.0);
    for (int i = 0; i < DETERMINISTIC_MAX_RAY_BOUNCES; i++) {
        if (false == hit_scene(current_ray, MAX_FLOAT)) {
            accumulated_radiance += BACKGROUND_COLOR * throughput;
            break;
        }

        // TODO: differentials should be recalculated for each bounce; using same data for rays other than first ray (from eye) is incorrect
        Material hit_material = hitMaterial;
        float3 hit_albedo = fetch_albedo(hitRec.local, current_ray.direction, hitRec.t, hit_material, incident.differentials);

        if (MATERIAL_LAMBERTIAN == hit_material.material_class) {
            accumulated_radiance += throughput * evaluate_dielectric_surface_color(camera_origin, hitRec, hit_material, hit_albedo);
            break;
        }

        if (MATERIAL_MIRROR == hit_material.material_class) {
            float3 reflected = evaluate_reflection(current_ray.direction, hitRec.global.normal, hitRec.global.position, hit_material.roughness);
            current_ray.origin = hitRec.global.position + reflected * SECONDARY_RAY_START_BIAS;
            current_ray.direction = reflected;
            throughput *= hit_albedo;
        } else if (MATERIAL_GLASS == hit_material.material_class) {
            bool stochastic = false;
            current_ray = glass_scatter(hitRec, hit_material.refractive_index_eta, current_ray.direction, stochastic);
            current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
            throughput *= hit_albedo;
        } else {
            accumulated_radiance += hit_albedo;
            break;
        }
    }

    return accumulated_radiance;
}

float3 evaluate_dielectric_surface_color(float3 camera_origin, HitRecord hit, Material hit_material, float3 hit_albedo) {
    float3 light_center = lights.Q + (lights.u + lights.v) * 0.5;
    float3 to_light = light_center - hit.global.position;
    float to_light_distance = length(to_light);
    float3 to_light_direction = to_light_distance > MIN_FLOAT ? to_light / to_light_distance : float3(0.0, 0.0, 0.0);
    float light_size = length(cross(lights.u, lights.v)) * DETERMINISTIC_SHADOW_LIGHT_SIZE_SCALE;

    float diffuse_fall_off = max(0.0, dot(hit.global.normal, to_light_direction));
    float3 to_camera_direction = normalize(camera_origin - hit.global.position);
    float3 reflected_light = reflect(-to_light_direction, hit.global.normal);
    float specular_fall_off = pow(max(0.0, dot(reflected_light, to_camera_direction)), 4.0) * diffuse_fall_off;

    //float shadow = evaluate_soft_shadow(hit.global.position, to_light_direction, light_size, DETERMINISTIC_SHADOW_START_BIAS, to_light_distance);
    float shadow = evaluate_hard_shadow(hit.global.position, to_light_direction, DETERMINISTIC_SHADOW_START_BIAS, to_light_distance);
    // shadow is in [0..1]: 0 is too dark -> linearly transform [0..1] into [K..1]
    float shadow_lightened = shadow * (1.0 - DETERMINISTIC_SHADOW_FLOOR) + DETERMINISTIC_SHADOW_FLOOR;
    float occlusion = approximate_ambient_occlusion(hit.global.position, hit.global.normal);

    float3 diffuse = diffuse_fall_off * hit_albedo * occlusion;
    float3 specular = specular_fall_off * hit_material.specular;
    float3 ambient = BACKGROUND_COLOR * hit_albedo * occlusion;
    float3 emissive = hit_material.emission.rgb;

    float3 light_color = materials[lights.material_id].emission.rgb;
    float3 reflected = lerp(diffuse, specular, hit_material.specular_strength);

    return reflected * light_color * shadow_lightened + ambient + emissive;
}

float3 evaluate_reflection(float3 incident, float3 normal, float3 hit_position, float roughness) {
    float3 perfect = reflect(incident, normal);

    if (near_zero_scalar(roughness)) {
        return perfect;
    }

    float3 randomization = reflection_roughness_addition(hit_position, incident.z) * roughness;
    return normalize(perfect + randomization);
}

/* The randomization code below is from https://www.shadertoy.com/view/3sc3z4
The rand() function declared above gives not so random values – routhness
looks bad with it. */

float3 reflection_roughness_addition(float3 position, float extra_seed) {
    float phi = rand_from_seed((rand_0_1() + position.x + 0.357) * extra_seed) * 2.0 * PI;
    float theta = acos(2.0 * rand_from_seed((rand_0_1() + position.y + 16.35647) * extra_seed) - 1.0);
    float x = sin(theta) * cos(phi);
    float y = sin(theta) * sin(phi);
    float z = cos(theta);
    return float3(x, y, z);
}

float rand_from_seed(float seed) {
    return frac(sin(seed) * 43758.5453123);
}

// 'to_light' expected to be normalized
float evaluate_hard_shadow(float3 position, float3 to_light, float min_ray_offset, float max_ray_offset) {
    if (hit_scene(Ray(position + to_light * min_ray_offset, to_light), max_ray_offset)) {
        if (any(hitMaterial.emission.rgb > float3(0.0, 0.0, 0.0))) {
            return 1.0;
        }
        return 0.0;
    }
    return 1.0;
}

// 'to_light' expected to be normalized
float evaluate_soft_shadow(float3 position, float3 to_light, float light_size, float min_ray_offset, float max_ray_offset) {
    float result = evaluate_hard_shadow(position, to_light, min_ray_offset, max_ray_offset); // or DETERMINISTIC_SHADOW_MARCHING_MAX
    float offset = min_ray_offset;
    float3 next_point = position + to_light * offset;
    for (int i = 0; i < DETERMINISTIC_SHADOW_RAY_MAX_STEPS; i++) {
        float signed_distance = sample_signed_distance(next_point, to_light);

        float candidate = signed_distance * light_size / offset;
        result = min(result, candidate);
        if(result < DETERMINISTIC_SHADOW_MARCHING_MIN) {
            break;
        }

        offset += clamp(signed_distance, DETERMINISTIC_SHADOW_MIN_STEP, DETERMINISTIC_SHADOW_MAX_STEP);
        if(offset > max_ray_offset) {
            break;
        }

        next_point = position + to_light * offset;
    }
    result = max(result, DETERMINISTIC_SHADOW_MARCHING_MIN);
    return smoothstep(DETERMINISTIC_SHADOW_MARCHING_MIN, DETERMINISTIC_SHADOW_MARCHING_MAX, result);
}

bool inside_aabb(float3 box_min, float3 box_max, float3 probe) {
    return all(probe >= box_min) && all(probe <= box_max);
}

// expected normalized 'direction'
float sample_signed_distance(float3 position, float3 direction) {
    float record = MAX_FLOAT;

    int node_index = 0;
    int max_index = int(uniforms.bvh_length);
    while (node_index < max_index && NULL_POINTER_LINK != node_index) {
        BvhNode node = bvh_inflated[node_index];
        if(inside_aabb(node.aabb_min, node.aabb_max, position)) {
            if (PRIMITIVE_TYPE_SDF == node.primitive_type) {
                Sdf sdf = sdf[node.primitive_index];
                float time = sdf_time[node.primitive_index];
                float candidate_distance = sample_signed_distance_function(sdf, position, direction, time);
                if (candidate_distance < record) {
                    record = candidate_distance;
                }
            }
            node_index++;
        } else {
            node_index = node.hit_miss_skip_link;
        }
    }

    return record;
}

// 'direction' is expected to be normalized
float sample_signed_distance_function(Sdf sdf, float3 position, float3 direction, float time) {
    float3x4 sdf_inverse_location = float3x4(sdf.inverse_location_col_0,sdf.inverse_location_col_1,sdf.inverse_location_col_2);
    float3 local_position = transform_point(sdf_inverse_location, position);
    float3 local_direction = normalize(transform_vector(to_mat3x3(sdf_inverse_location), direction));
    float local_distance = sample_sdf(sdf, local_position, time);
    float3 local_next = local_position + local_direction * local_distance;

    float3x4 sdf_location = float3x4(sdf.location_col_0,sdf.location_col_1,sdf.location_col_2);
    float3 global_next = transform_point(sdf_location, local_next);
    float3 global_offset = global_next - position;
    float global_distance = length(global_offset) * sign(dot(global_offset, direction));

    return global_distance;
}

// 'normal' is expected to be normalized
float approximate_ambient_occlusion(float3 posision, float3 normal) {
    float occlusion = 0.0;
    float fall_off = 1.0;
    for(int i = 0; i < DETERMINISTIC_AMBIENT_OCCLUSION_SAMPLES; i++) {
        float height = 0.01 + 0.12 * float(i) / 4.0;
        float signed_distance = sample_signed_distance(posision + height * normal, normal);
        occlusion += max(0.0, (height - signed_distance) * fall_off);
        fall_off *= 0.95; // smaller values - lighter the shadow
        if(occlusion > 0.35) {
            break;
        }
    }

    // smaller values makes shadow outstand further from the caster; must be synced with aabb inflate ratio
    static const float OVERALL_INVERTED_SHADOW_LENGTH = 2.5;
    // bigger values makes shadow darker
    static const float SHADOW_DARKNESS = 7.0;

    return clamp(OVERALL_INVERTED_SHADOW_LENGTH - SHADOW_DARKNESS * occlusion, 0.0, 1.0);
}