const PI: f32 = 3.1415926535897932385;

const MIN_FLOAT: f32 = 0.0001;
const MAX_FLOAT: f32 = 999999999.999;

const SECONDARY_RAY_START_BIAS: f32 = 0.0005;
const RAY_PARAMETER_MIN: f32 = 0.000001;

const MATERIAL_LAMBERTIAN: i32 = 0;
const MATERIAL_MIRROR: i32 = 1;
const MATERIAL_GLASS: i32 = 2;
const MATERIAL_ISOTROPIC: i32 = 3;

const TEXTURE_WRAP_MODE_REPEAT: i32 = 0;
const TEXTURE_WRAP_MODE_CLAMP: i32 = 1;
// const TEXTURE_WRAP_MODE_DISCARD: i32 = 2; - default behavior if the mode is none of the above

const PRIMITIVE_TYPE_SDF: u32 = 1;
const PRIMITIVE_TYPE_TRIANGLE: u32 = 2;

const NULL_POINTER_LINK: i32 = -1;

const WORK_GROUP_SIZE_X: u32 = 8;
const WORK_GROUP_SIZE_Y: u32 = 8;
const WORK_GROUP_SIZE_Z: u32 = 1;
const WORK_GROUP_SIZE = vec3<u32>(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, WORK_GROUP_SIZE_Z);

const BACKGROUND_COLOR = vec3f(0.1);

const DETERMINISTIC_AMBIENT_OCCLUSION_SAMPLES = 5;
const DETERMINISTIC_SHADOW_RAY_MAX_STEPS = 32;
const DETERMINISTIC_SHADOW_MIN_STEP = 0.01;
const DETERMINISTIC_SHADOW_MAX_STEP = 0.1;
const DETERMINISTIC_SHADOW_START_BIAS = 0.005;
const DETERMINISTIC_SHADOW_LIGHT_SIZE_SCALE = 8.0; // bigger - crispier shadows
const DETERMINISTIC_SHADOW_MARCHING_MIN = -1.0;
const DETERMINISTIC_SHADOW_MARCHING_MAX = 1.0;
const DETERMINISTIC_SHADOW_FLOOR = 0.6;
const DETERMINISTIC_MAX_RAY_BOUNCES = 8;

const MONTE_CARLO_MAX_RAY_BOUNCES = 50;
const MONTE_CARLO_STRATIFY_SAMLING = false;
const MONTE_CARLO_IMPORTANCE_SAMPLING = true;
const MAX_SDF_RAY_MARCH_STEPS = 120;

@group(0) @binding( 0) var<uniform> uniforms : Uniforms;

@group(0) @binding( 1) var atlases_sampler: sampler;
@group(0) @binding( 2) var texture_atlas_page: texture_2d<f32>;

@group(1) @binding( 0) var<storage, read_write> pixel_color_buffer: array<vec4f>;
@group(1) @binding( 1) var<storage, read_write> object_id_buffer: array<u32>;
@group(1) @binding( 2) var<storage, read_write> normal_buffer: array<vec4f>;
@group(1) @binding( 3) var<storage, read_write> albedo_buffer: array<vec4f>;

@group(2) @binding( 0) var<storage, read> quad_objs: array<Parallelogram>;
@group(2) @binding( 1) var<storage, read> sdf: array<Sdf>;
@group(2) @binding( 2) var<storage, read> triangles: array<Triangle>;
@group(2) @binding( 3) var<storage, read> materials: array<Material>;
@group(2) @binding( 4) var<storage, read> bvh: array<BvhNode>;
@group(2) @binding( 5) var<storage, read> bvh_inflated: array<BvhNode>;
@group(2) @binding( 6) var<storage, read> sdf_time: array<f32>;
@group(2) @binding( 7) var<storage, read> texture_atlases_mapping: array<AtlasMapping>;

var<private> randState: u32 = 0u;

var<private> hitRec: HitRecord;
var<private> hitMaterial: Material;
var<private> scatterRec: ScatterRecord;
var<private> lights: Parallelogram;

struct Uniforms {
	frame_buffer_size: vec2u,
	frame_buffer_area: u32,
	frame_buffer_aspect: f32, // width / height
	inverted_frame_buffer_size: vec2f,
	frame_number: f32,
	empty_slot: f32,
	view_matrix: mat4x4f,
	/* Consider a view ray defined by an origin (e.g., the eye position for a perspective camera)
    and a direction that intersects the view plane at a world-space pixel position.
    This matrix, when multiplied by the world-space pixel position, returns the ray's origin.
    For a perspective camera, the origin is always the eye position — the same for all pixels.
    For an orthographic camera, the origin lies on the camera plane and varies per pixel. */
	view_ray_origin_matrix : mat4x4f,

	parallelograms_count: u32,
	bvh_length: u32,
	pixel_side_subdivision: u32, // anti-aliasing level: bigger value -> slower render -> less jagged edges
	global_time_seconds: f32,
}

struct Ray {
	origin: vec3f,
	direction: vec3f,
}

struct Material {
	albedo: vec3f,
	specular: vec3f,
	emission: vec3f,
	specular_strength: f32, // chance that a ray hitting would reflect specularly
	roughness: f32, // diffuse strength
	refractive_index_eta: f32, // refractive index
	albedo_texture_uid: i32, // > 0 - atlas r_e_g_i_o_n index (1-based), < 0 - procedural texture uid, = 0 - none
	material_class: i32,
}

struct AtlasMapping {
    top_left_corner_uv: vec2f,
    size: vec2f,
    local_position_to_texture: mat2x4<f32>,
    wrap_mode: vec2i,
}

struct Parallelogram {
	Q: vec3f,
	u: vec3f,
	object_uid: u32,
	v: vec3f,
	D: f32,
	normal: vec3f,
	w: vec3f,
	material_id: u32,
}

struct Triangle {
	A : vec3f,
	B : vec3f,
	C : vec3f,
	normalA : vec3f,
	normalB : vec3f,
	object_uid : u32,
	normalC : vec3f,
	material_id : u32,
}

struct Sdf {
    location : mat3x4f,
    inverse_location : mat3x4f,
    ray_marching_step_scale: f32,
    class_index : i32,
    material_id : u32,
    object_uid : u32,
}

struct BvhNode {
	aabb_min : vec3f,
	primitive_index: u32,

	aabb_max: vec3f,
	primitive_type: u32,

	hit_miss_skip_link: i32,
}

struct HitPlace {
    position: vec3f,
    normal: vec3f,
}

struct HitRecord {
	global: HitPlace,
	local: HitPlace,
	t: f32,
	material_id: u32,
	front_face: bool,
}

struct FirstHitSurface {
	object_uid : u32,
	albedo : vec3f,
	normal : vec3f,
}

struct ScatterRecord {
	pdf : f32,
	skip_pdf : bool,
	skip_pdf_ray : Ray
}

fn at(ray: Ray, t: f32) -> vec3f {
	return ray.origin + t * ray.direction;
}

// PCG prng
// https://www.shadertoy.com/view/XlGcRh
fn rand_0_1() -> f32 {
	randState = randState * 747796405u + 2891336453u;
	var word: u32 = ((randState >> ((randState >> 28u) + 4u)) ^ randState) * 277803737u;
	return f32((word >> 22u)^word) / 4294967295;
}

fn random_double(min : f32, max : f32) -> f32 {
	return min + (max - min) * rand_0_1();
}

@must_use
fn near_zero(v : vec3f) -> bool {
	return near_zero_scalar(v[0]) && near_zero_scalar(v[1]) && near_zero_scalar(v[2]);
}

@must_use
fn near_zero_scalar(v : f32) -> bool {
	return abs(v) < MIN_FLOAT;
}

@must_use
fn sample_sdf(sdf: Sdf, point: vec3f, time: f32) -> f32 {
    return sdf_select(sdf.class_index, point, time);
}

@must_use
fn apply_animation(sdf: Sdf, point: vec3f, time: f32) -> vec3f {
    return sdf_apply_animation(sdf.class_index, point, time);
}

@must_use
fn signed_distance_normal(sdf: Sdf, point: vec3f, time: f32) -> vec3f {
    let e = vec2f(1.0,-1.0)*0.5773*0.0005;
    return normalize( e.xyy * sample_sdf( sdf, point + e.xyy, time ) +
					  e.yyx * sample_sdf( sdf, point + e.yyx, time ) +
					  e.yxy * sample_sdf( sdf, point + e.yxy, time ) +
					  e.xxx * sample_sdf( sdf, point + e.xxx, time ) );
}

@must_use
fn transform_point(transformation: mat3x4f, point: vec3f) -> vec3f {
    return vec4f(point, 1.0f) * transformation;
}

@must_use
fn to_mat3x3(source: mat3x4f) -> mat3x3f {
    return mat3x3f(source[0].xyz, source[1].xyz, source[2].xyz);
}

@must_use
fn transform_vector(transformation: mat3x3f, vector: vec3f) -> vec3f {
    return vector * transformation;
}

@must_use
fn transform_transposed_vector(transformation: mat3x3f, vector: vec3f) -> vec3f {
    /*Sdf matrices come from CPU in row-major format, so
    we need to multiply like v * M. Swaping operands like
    M * v equals to v * transpose(M).*/
    return transformation * vector;
}

@must_use
fn transform_ray_parameter(transformation: mat3x4f, ray: Ray, parameter: f32, transformed_origin: vec3f) -> f32 {
    let point = transform_point(transformation, at(ray, parameter));
    return length(point - transformed_origin);
}

@must_use
fn hit_sdf(sdf: Sdf, time: f32, ray: Ray, tmin: f32, tmax: f32) -> bool {
    let sdf_location_inverse = to_mat3x3(sdf.inverse_location);
    let local_ray_origin = transform_point(sdf.inverse_location, ray.origin);
    let local_ray_direction = normalize(transform_vector(sdf_location_inverse, ray.direction));
    let local_ray = Ray(local_ray_origin, local_ray_direction);

    var local_t = transform_ray_parameter(sdf.inverse_location, ray, tmin, local_ray_origin);
    var local_t_max = transform_ray_parameter(sdf.inverse_location, ray, tmax, local_ray_origin);

    var i: i32 = 0;
    loop {
        if (i >= MAX_SDF_RAY_MARCH_STEPS) {
            break;
        }
        if (local_t>local_t_max) {
            break;
        }

        let candidate = at(local_ray, local_t);
        let signed_distance = sample_sdf(sdf, candidate, time);
        let t_scaled = 0.0001 * local_t;

        if(abs(signed_distance) < t_scaled) {
            hitRec.local.normal = signed_distance_normal(sdf, candidate, time);
            hitRec.global.normal = normalize(transform_transposed_vector(sdf_location_inverse, hitRec.local.normal));

            hitRec.global.position = transform_point(sdf.location, candidate);
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

        let step_size = max(abs(signed_distance) * sdf.ray_marching_step_scale, t_scaled);
        local_t += step_size;
        i = i + 1;
    }

    return false;
}

fn hit_quad(quad : Parallelogram, tmin : f32, tmax : f32, ray : Ray) -> bool {

	if(dot(ray.direction, quad.normal) > 0) {
		return false;
	}

	let denom = dot(quad.normal, ray.direction);

	// no hit if the ray is paraller to the plane
	if(abs(denom) < 1e-8) {
		return false;
	}

	let t = (quad.D - dot(quad.normal, ray.origin)) / denom;
	if(t <= tmin || t >= tmax) {
		return false;
	}

	// determine if hit point lies within quarilateral
	let intersection = at(ray, t);
	let planar_hitpt_vector = intersection - quad.Q;
	let alpha = dot(quad.w, cross(planar_hitpt_vector, quad.v));
	let beta = dot(quad.w, cross(quad.u, planar_hitpt_vector));

	if(alpha < 0 || 1 < alpha || beta < 0 || 1 < beta) {
		return false;
	}

	hitRec.t = t;

    let local_position = quad.u * alpha + quad.v * beta;
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
fn hit_triangle(triangle: Triangle, tmin: f32, tmax: f32, ray: Ray) -> bool {
	let AB = triangle.B - triangle.A;
	let AC = triangle.C - triangle.A;
	let normal = cross(AB, AC);
	let determinant = -dot(ray.direction, normal);

	// CULLING
	if(abs(determinant) < tmin) {
		return false;
	}

	let ao = ray.origin - triangle.A;
	let dao = cross(ao, ray.direction);

	// calculate dist to triangle & barycentric coordinates of intersection point
	let invDet = 1.0 / determinant;
	let dst = dot(ao, normal) * invDet;
	let u = dot(AC, dao) * invDet;
	let v = -dot(AB, dao) * invDet;
	let w = 1 - u - v;

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
    hit: bool,
    ray_parameter: f32,
}

// https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
fn hit_aabb(box_min: vec3f, box_max: vec3f, tmin: f32, tmax: f32, ray_origin: vec3f, inverted_ray_dir: vec3f) -> AabbHit {
	var t0s = (box_min - ray_origin) * inverted_ray_dir;
	var t1s = (box_max - ray_origin) * inverted_ray_dir;

	var tsmaller = min(t0s, t1s);
	var tbigger = max(t0s, t1s);

	var t_min = max(tmin, max(tsmaller.x, max(tsmaller.y, tsmaller.z)));
	var t_max = min(tmax, min(tbigger.x, min(tbigger.y, tbigger.z)));

	return AabbHit(t_max > t_min, t_min);
}

fn get_lights() {
	for(var i = u32(0); i < uniforms.parallelograms_count; i++) {
		let emission = materials[quad_objs[i].material_id].emission;

		if(emission.x > 0.0) {
			lights = quad_objs[i];
			break;
		}
	}
}

fn evaluate_pixel_index(global_invocation_id: vec3<u32>, workgroups_count: vec3<u32>) -> u32 {
    let grid_dimension = WORK_GROUP_SIZE * workgroups_count;
    return
        global_invocation_id.z * (grid_dimension.x * grid_dimension.y) +
        global_invocation_id.y * (grid_dimension.x) +
        global_invocation_id.x ;
}

struct Pixel {
    coordinates: vec2f
}

@must_use
fn setup_pixel_coordinates(pixel_index: u32) -> Pixel {
    let x: u32 = pixel_index % uniforms.frame_buffer_size.x;
    let y: u32 = pixel_index / uniforms.frame_buffer_size.x;
    return Pixel(vec2f(f32(x), f32(y)));
}

@must_use
fn setup_camera() -> Camera {
    let fov_factor = 1.0 / tan(60 * (PI / 180.0) / 2.0);
	let origin = uniforms.view_matrix[3].xyz;
	return Camera(fov_factor, origin);
}

@must_use
fn pixel_outside_frame_buffer(pixel_index: u32) -> bool {
    return pixel_index >= uniforms.frame_buffer_area;
}

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_surface_attributes_buffer(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

	let pixel = setup_pixel_coordinates(pixel_index);
	let camera = setup_camera();

    let ray = ray_and_differentials(camera, pixel, 0.5, 0.5);
	let surface_intersection = trace_first_intersection(ray);
    object_id_buffer[pixel_index] = surface_intersection.object_uid;
    albedo_buffer[pixel_index] = vec4f(surface_intersection.albedo, 1.0f);
    normal_buffer[pixel_index] = vec4f(surface_intersection.normal, 0.0f);
}

@must_use
fn make_common_color_evaluation_setup(pixel_index: u32) -> Pixel {
    let pixel = setup_pixel_coordinates(pixel_index);
	get_lights();
	return pixel;
}

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_color_buffer_monte_carlo(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

    let camera = setup_camera();
	let pixel = make_common_color_evaluation_setup(pixel_index);

	randState = pixel_index + u32(uniforms.frame_number) * 719393;
	let traced_color = path_trace_monte_carlo(camera, pixel);

    pixel_color_buffer[pixel_index] = vec4f(pixel_color_buffer[pixel_index].xyz + traced_color, 1.0);
}

struct Camera {
    fov_factor: f32,
    origin: vec3f,
}

/*
x = aspect * (2 * (x / width) - 1) 	[ranges from -aspect to +aspect]
y = -(2 * (y / height) - 1)			[ranges from +1 to -1]
lower left pixel corner -> 0.5, 0.5 gives pixel's center;
lower left pixel corner -> 0.5, 0.5 gives pixel's center;
*/
@must_use
fn ray_to_pixel(camera: Camera, pixel: Pixel, sub_pixel_x: f32, sub_pixel_y: f32) -> Ray {
    let s = uniforms.frame_buffer_aspect * (2 * ((pixel.coordinates.x + sub_pixel_x) * uniforms.inverted_frame_buffer_size.x) - 1);
    let t = -1 * (2 * ((pixel.coordinates.y + sub_pixel_y) * uniforms.inverted_frame_buffer_size.y) - 1);
    return get_camera_ray(camera, s, t);
}

@must_use
fn get_camera_ray(camera: Camera, s : f32, t : f32) -> Ray {
	let eye_to_pixel_direction = (uniforms.view_matrix * vec4f(vec3f(s, t, -camera.fov_factor), 0.0)).xyz;
	let pixel_world_space = vec4(camera.origin + eye_to_pixel_direction, 1.0);

	let ray_origin_world_space = (uniforms.view_ray_origin_matrix * pixel_world_space).xyz;
	let direction = normalize(pixel_world_space.xyz - ray_origin_world_space);

	return Ray(ray_origin_world_space, direction);
}

struct RayDifferentials {
    dx: vec3f,
    dy: vec3f,
}

@must_use
fn ray_differentials(camera: Camera, pixel: Pixel, sub_pixel_x: f32, sub_pixel_y: f32) -> RayDifferentials {
    let ray_direction_dx = ray_to_pixel(camera, Pixel(pixel.coordinates+vec2f(1.0, 0.0)), sub_pixel_x, sub_pixel_y);
    let ray_direction_dy = ray_to_pixel(camera, Pixel(pixel.coordinates+vec2f(0.0, 1.0)), sub_pixel_x, sub_pixel_y);
    return RayDifferentials(ray_direction_dx.direction, ray_direction_dy.direction);
}

struct RayAndDifferentials {
    ray: Ray,
    differentials: RayDifferentials,
}

@must_use
fn ray_and_differentials(camera: Camera, pixel: Pixel, sub_pixel_x: f32, sub_pixel_y: f32) -> RayAndDifferentials {
    let ray = ray_to_pixel(camera, pixel, sub_pixel_x, sub_pixel_y);
    let differentials = ray_differentials(camera, pixel, sub_pixel_x, sub_pixel_y);
    return RayAndDifferentials(ray, differentials);
}

// position partial derivatives (with respect to x and y axes)
struct RayDerivatives {
    dp_dx: vec3f,
    dp_dy: vec3f,
}

/*
We have a ray (O, d̂) that intersects a surface at point P. There is also a second ray (O, d̂ₓ).
The surface at point P has a normal N — we’ll assume the surface is a plane (with that same
normal N and passing through P). For smooth surfaces, in a sufficiently small neighborhood
of P, this is actually not far from reality.

Let’s denote the intersection of the second ray with this plane as Pₓ. What we need is the
vector (Pₓ - P) — this will be an approximation of the footprint that the pixel (through which
the original ray passed) has on the surface.

P = O + t * d̂ where t is a scalar.
Pₓ = O + tₓ * d̂ₓ where tₓ is some other scalar (which we don’t know).

All points lying on our plane satisfy the equation (?, N) - (P, N) = 0.
Let’s plug Pₓ and P into this:

(O + tₓ * d̂ₓ - P, N) = 0
(O, N) + tₓ * (d̂ₓ, N) - (P, N) = 0
(O, N) + tₓ * (d̂ₓ, N) - (O + t * d̂, N) = 0
(O, N) + tₓ * (d̂ₓ, N) - (O, N) - t * (d̂, N) = 0
tₓ * (d̂ₓ, N) - t * (d̂, N) = 0
tₓ = t * (d̂, N) / (d̂ₓ, N)

Now:

Pₓ - P =
O + tₓ * d̂ₓ - O - t * d̂ =
tₓ * d̂ₓ - t * d̂ =
t * ((d̂, N) / (d̂ₓ, N) * d̂ₓ - d̂)

So, Pₓ - P = t * ((d̂, N) / (d̂ₓ, N) * d̂ₓ - d̂)
*/

@must_use
fn ray_hit_position_derivatives(
    ray_direction: vec3f,
    surface_intersection_parameter: f32,
    surface_normal: vec3f,
    ray_differentials: RayDifferentials,
) -> RayDerivatives {
    let ray_dot_normal = dot(ray_direction, surface_normal);
    let dp_dx = surface_intersection_parameter * (ray_differentials.dx * ray_dot_normal / dot(ray_differentials.dx, surface_normal) - ray_direction);
    let dp_dy = surface_intersection_parameter * (ray_differentials.dy * ray_dot_normal / dot(ray_differentials.dy, surface_normal) - ray_direction);
    return RayDerivatives(dp_dx, dp_dy);
}

@must_use
fn path_trace_monte_carlo(camera: Camera, pixel: Pixel) -> vec3f {
	let samples_count = uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision;
	var result_color = vec3f(0.0);
	if(MONTE_CARLO_STRATIFY_SAMLING) {
		let reciprocal_sqrt_samples_per_pixel = 1.0 / f32(uniforms.pixel_side_subdivision);
		for(var i = u32(0); i < uniforms.pixel_side_subdivision; i++) {
			for(var j = u32(0); j < uniforms.pixel_side_subdivision; j++) {
                let sub_pixel_x = reciprocal_sqrt_samples_per_pixel * (f32(i) + rand_0_1());
                let sub_pixel_y = reciprocal_sqrt_samples_per_pixel * (f32(j) + rand_0_1());
                let ray = ray_and_differentials(camera, pixel, sub_pixel_x, sub_pixel_y);
				result_color += ray_color_monte_carlo(ray);
			}
		}
	} else {
		for(var i = u32(0); i < samples_count; i++) {
		    let sub_pixel_x = rand_0_1();
		    let sub_pixel_y = rand_0_1();
		    let ray = ray_and_differentials(camera, pixel, sub_pixel_x, sub_pixel_y);
			result_color += ray_color_monte_carlo(ray);
		}
	}
	result_color /= f32(samples_count);

	return result_color;
}

@must_use
fn trace_first_intersection(incident : RayAndDifferentials) -> FirstHitSurface {
    let ray = incident.ray;

    var closest_so_far = MAX_FLOAT;
    var hit_uid: u32 = 0;
    var hit_material_id: u32 = 0;
    var hit_global_normal: vec3f = vec3f(0.0f);
    var hit_local: HitPlace = HitPlace(vec3f(0.0f), vec3f(0.0f));

    for(var i = u32(0); i < uniforms.parallelograms_count; i++){
        let parallelogram = quad_objs[i];
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
        let inverted_ray_dir = 1.0 / ray.direction;
        var node_index: i32 = 0;
        let max_index = i32(uniforms.bvh_length);
        while (node_index < max_index && NULL_POINTER_LINK != node_index) {
            let node = bvh[node_index];
            let aabb_hit = hit_aabb(node.aabb_min, node.aabb_max, RAY_PARAMETER_MIN, closest_so_far, ray.origin, inverted_ray_dir);
            if(aabb_hit.hit) {
                if(PRIMITIVE_TYPE_TRIANGLE == node.primitive_type) {
                    let triangle = triangles[node.primitive_index];
                    if(hit_triangle(triangle, RAY_PARAMETER_MIN, closest_so_far, ray)) {
                        hit_uid = triangle.object_uid;
                        hit_material_id = triangle.material_id;
                        hit_global_normal = hitRec.global.normal;
                        hit_local = hitRec.local;
                        closest_so_far = hitRec.t;
                    }
                } else if (PRIMITIVE_TYPE_SDF == node.primitive_type) {
                    let sdf = sdf[node.primitive_index];
                    if(hit_sdf(sdf, sdf_time[node.primitive_index], ray, aabb_hit.ray_parameter, closest_so_far)) {
                        hit_uid = sdf.object_uid;
                        hit_material_id = sdf.material_id;
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

    var hit_albedo: vec3f;
    if (0 < hit_uid) {
        hit_albedo = fetch_albedo(hit_local, incident.ray.direction, closest_so_far, materials[hit_material_id], incident.differentials);
    } else {
        hit_albedo = vec3f(0.0);
    }

    return FirstHitSurface(hit_uid, hit_albedo, hit_global_normal);
}

@must_use
fn snap_to_grid(victim: vec3f, grid_step: f32) -> vec3f {
    return floor((victim - grid_step * sign(victim)) / vec3f(grid_step)) * grid_step;
}

@must_use
fn fetch_albedo(hit: HitPlace, ray_direction: vec3f, ray_parameter: f32, material: Material, differentials: RayDifferentials) -> vec3f {
    var result = material.albedo;
    if (material.albedo_texture_uid < 0) {
        /*
        Grid snapping reduces visual flickering caused by floating-point precision issues
        during ray-surface intersection. The problem is especially pronounced when using
        discontinuous procedural textures (e.g., checkerboard patterns), where tiny
        differences in hit positions—particularly along cube edges and corners—can cause
        abrupt texture color changes.

        By snapping the hit position to a small 3D grid (via quantization), we ensure
        that nearby intersection points consistently evaluate to the same texture value,
        reducing instability in rendered outpu
        */
        const grid_step: f32 = 1e-4;
        let snapped_position = snap_to_grid(hit.position, grid_step);
        let derivartives = ray_hit_position_derivatives(ray_direction, ray_parameter, hit.normal, differentials);

        result *= procedural_texture_select(
            -material.albedo_texture_uid,
            snapped_position, hit.normal,
            uniforms.global_time_seconds,
            derivartives.dp_dx,
            derivartives.dp_dy
        );
    } else if (material.albedo_texture_uid > 0) {
        let region_index = material.albedo_texture_uid - 1;
        let atlas_region_mapping = texture_atlases_mapping[region_index];
        let derivartives = ray_hit_position_derivatives(ray_direction, ray_parameter, hit.normal, differentials);

        let texture_sample = read_atlas(hit.position, atlas_region_mapping, derivartives);
        result = (1.0 - texture_sample.a) * result + texture_sample.a * texture_sample.rgb;
    }
    return result;
}

@must_use
fn read_atlas(local_space_position: vec3f, atlas_region_mapping: AtlasMapping, differentials: RayDerivatives) -> vec4f {
    var texture_coordinate = vec4f(local_space_position, 1.0) * atlas_region_mapping.local_position_to_texture;
    let ddx = vec4f(differentials.dp_dx, 0.0) * atlas_region_mapping.local_position_to_texture;
    let ddy = vec4f(differentials.dp_dy, 0.0) * atlas_region_mapping.local_position_to_texture;

    for (var i = 0; i < 2; i++) {
        let coordinate = texture_coordinate[i];
        let mode = atlas_region_mapping.wrap_mode[i];
        let inset = pixel_half_size(texture_atlas_page, ddx, ddy)[i];
        let min_edge = inset/atlas_region_mapping.size[i];
        let max_edge = 1.0 - inset/atlas_region_mapping.size[i];
        if (TEXTURE_WRAP_MODE_REPEAT == mode) {
            texture_coordinate[i] = fract(coordinate);
        } else if (TEXTURE_WRAP_MODE_CLAMP == mode) {
            texture_coordinate[i] = clamp(coordinate, min_edge, max_edge);
        } else {
            if (coordinate < min_edge || coordinate > max_edge) {
                return vec4f(0.0);
            }
        }
    }

    let uv = atlas_region_mapping.top_left_corner_uv + texture_coordinate * atlas_region_mapping.size;
    let texture_sample = textureSampleGrad(texture_atlas_page, atlases_sampler, uv, ddx, ddy);

    return texture_sample;
}

@must_use
fn calculate_mip_level(target_texture: texture_2d<f32>, ddx: vec2<f32>, ddy: vec2<f32>) -> u32 {
    let texture_size = vec2<f32>(textureDimensions(target_texture, 0));

    // uv gradients -> texel gradients
    let ddx_texel = ddx * texture_size;
    let ddy_texel = ddy * texture_size;

    // maximum change per pixel
    let delta_max_sqr = max(length(ddx_texel), length(ddy_texel));

    if (delta_max_sqr <= 0.0) {
        return 0;
    }

    // calculate mip level using the same formula as GPU hardware
    let mip_level = u32(0.5 * log2(delta_max_sqr));

    return clamp(mip_level, 0, textureNumLevels(target_texture) - 1);
}

@must_use
fn pixel_half_size(target_texture: texture_2d<f32>, ddx: vec2<f32>, ddy: vec2<f32>) -> vec2f {
   let mip_level = calculate_mip_level(target_texture, ddx, ddy);
   let mip_size = textureDimensions(target_texture, mip_level);
   return 0.5 / vec2<f32>(mip_size);
}

fn hit_scene(ray: Ray, max_ray_patameter: f32) -> bool {
	var closest_so_far = max_ray_patameter;
	var hit_anything = false;

	for(var i = u32(0); i < uniforms.parallelograms_count; i++) {
		if(hit_quad(quad_objs[i], RAY_PARAMETER_MIN, closest_so_far, ray)) {
			hit_anything = true;
			closest_so_far = hitRec.t;
		}
	}

    // BVH traversal from "Implementing a practical rendering system using GLSL" by Toshiya Hachisuka
	{
        let inverted_ray_dir = 1.0 / ray.direction;
        var node_index: i32 = 0;
        let max_index = i32(uniforms.bvh_length);
        while (node_index < max_index && NULL_POINTER_LINK != node_index) {
            let node = bvh[node_index];
            let aabb_hit = hit_aabb(node.aabb_min, node.aabb_max, RAY_PARAMETER_MIN, closest_so_far, ray.origin, inverted_ray_dir);
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

fn ray_color_monte_carlo(incident : RayAndDifferentials) -> vec3f {

	var current_ray = incident.ray;
	var accumulated_radiance = vec3f(0.0);
	var throughput = vec3f(1.0);

	for(var i = 0; i < MONTE_CARLO_MAX_RAY_BOUNCES; i++) {
		if(hit_scene(current_ray, MAX_FLOAT) == false) {
			accumulated_radiance += BACKGROUND_COLOR * throughput;
			break;
		}

        // TODO: differentials should be recalculated for each bounce; using same data for rays other than first ray (from eye) is incorrect
        let albedo_color = fetch_albedo(hitRec.local, current_ray.direction, hitRec.t, hitMaterial, incident.differentials);
		var emission_color = hitMaterial.emission;
		if(!hitRec.front_face) {
			emission_color = vec3f(0.0);
		}

		if(MONTE_CARLO_IMPORTANCE_SAMPLING) {
			let scatterred_surface = material_scatter(current_ray);

			if(scatterRec.skip_pdf) {
				accumulated_radiance += emission_color * throughput;
				throughput *= mix(albedo_color, hitMaterial.specular, doSpecular);

				current_ray = scatterRec.skip_pdf_ray;
				current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
				continue;
			}

            const LIGHT_SAMPLING_PROBABILITY = 0.2;
			var scattered: Ray;
			if(rand_0_1() > LIGHT_SAMPLING_PROBABILITY) {
				scattered = scatterred_surface;
			} else {
			    scattered = get_random_on_quad(lights, hitRec.global.position);
			}

			let lambertian_pdf = onb_lambertian_scattering_pdf(scattered);
			let light_pdf = light_pdf(scattered, lights);
			let pdf = LIGHT_SAMPLING_PROBABILITY * light_pdf + (1.0 - LIGHT_SAMPLING_PROBABILITY) * lambertian_pdf;

			if(pdf <= 0.00001) {
				return emission_color * throughput;
			}

			accumulated_radiance += emission_color * throughput;
			throughput *= ((lambertian_pdf * mix(albedo_color, hitMaterial.specular, doSpecular)) / pdf);
			current_ray = scattered;
			current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
		} else {
			let scattered = material_scatter(current_ray);

			accumulated_radiance += emission_color * throughput;
			throughput *= mix(albedo_color, hitMaterial.specular, doSpecular);

			current_ray = scattered;
			current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
		}

		// russian roulette
		if(i > 2) {
			let p = max(throughput.x, max(throughput.y, throughput.z));
			if(rand_0_1() > p) {
				break;
			}

			throughput *= (1.0 / p);
		}
	}

	return accumulated_radiance;
}

var<private> doSpecular : f32;
fn material_scatter(ray_in : Ray) -> Ray {
	var scattered = Ray(vec3f(0.0), vec3f(0.0));
	doSpecular = 0;
	if(MATERIAL_LAMBERTIAN == hitMaterial.material_class) {

		let uvw = onb_build_from_w(hitRec.global.normal);
		var diffuse_dir = cosine_sampling_wrt_Z();
		diffuse_dir = normalize(onb_get_local(diffuse_dir));

		scattered = Ray(hitRec.global.position, diffuse_dir);

		doSpecular = select(0.0, 1.0, rand_0_1() < hitMaterial.specular_strength);

		var specular_dir = reflect(ray_in.direction, hitRec.global.normal);
		specular_dir = normalize(mix(specular_dir, diffuse_dir, hitMaterial.roughness));

		scattered = Ray(hitRec.global.position, normalize(mix(diffuse_dir, specular_dir, doSpecular)));

		scatterRec.skip_pdf = false;

		if(doSpecular == 1.0) {
			scatterRec.skip_pdf = true;
			scatterRec.skip_pdf_ray = scattered;
		}
	}
	else if(MATERIAL_MIRROR == hitMaterial.material_class) {
		var reflected = reflect(ray_in.direction, hitRec.global.normal);
		scattered = Ray(hitRec.global.position, normalize(reflected + hitMaterial.roughness * uniform_random_in_unit_sphere()));

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}
	else if(MATERIAL_GLASS == hitMaterial.material_class) {
		let stochastic = true;
		scattered = glass_scatter(hitRec, hitMaterial.refractive_index_eta, ray_in.direction, stochastic);

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}
	else if(MATERIAL_ISOTROPIC == hitMaterial.material_class) {
		let g = hitMaterial.specular_strength;
		let cos_hg = (1 + g*g - pow(((1 - g*g) / (1 - g + 2*g*rand_0_1())), 2.0)) / (2 * g);
		let sin_hg = sqrt(1 - cos_hg * cos_hg);
		let phi = 2 * PI * rand_0_1();

		let hg_dir = vec3f(sin_hg * cos(phi), sin_hg * sin(phi), cos_hg);

		let uvw = onb_build_from_w(ray_in.direction);
		scattered = Ray(hitRec.global.position, normalize(onb_get_local(hg_dir)));

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}

	return scattered;
}

@must_use
fn glass_scatter(hit: HitRecord, refractive_index_eta: f32, in_ray_direction: vec3f, stochastic: bool) -> Ray {
    var ir = refractive_index_eta;
    if(hit.front_face) {
        ir = (1.0 / ir);
    }

    let unit_direction = in_ray_direction;
    let cos_theta = min(-dot(unit_direction, hit.global.normal), 1.0);
    let sin_theta = sqrt(1 - cos_theta*cos_theta);

    var direction = vec3f(0.0);
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

    return Ray(hitRec.global.position, direction);
}

fn reflectance(cosine : f32, ref_idx : f32) -> f32 {
	var r0 = (1 - ref_idx) / (1 + ref_idx);
	r0 = r0 * r0;
	return r0 + (1 - r0) * pow((1 - cosine), 5.0);
}

fn uniform_random_in_unit_sphere() -> vec3f {
	let phi = rand_0_1() * 2.0 * PI;
	let theta = acos(2.0 * rand_0_1() - 1.0);

	let x = sin(theta) * cos(phi);
	let y = sin(theta) * sin(phi);
	let z = cos(theta);

	return vec3f(x, y, z);
}

fn random_in_unit_disk() -> vec3f {
	let theta = 2 * PI * rand_0_1();
	return vec3f(cos(theta), sin(theta), 0);
}

fn uniform_sampling_hemisphere() -> vec3f {
    let on_unit_sphere = uniform_random_in_unit_sphere();
	let sign_dot = select(1.0, 0.0, dot(on_unit_sphere, hitRec.global.normal) > 0.0);
    return normalize(mix(on_unit_sphere, -on_unit_sphere, sign_dot));
}

fn cosine_sampling_hemisphere() -> vec3f {
	return uniform_random_in_unit_sphere() + hitRec.global.normal;
}

// generates a random direction weighted by PDF = cos_theta / PI relative to z axis
fn cosine_sampling_wrt_Z() -> vec3f {
	let r1 = rand_0_1();
	let r2 = rand_0_1();

	let phi = 2 * PI * r1;
	let x = cos(phi) * sqrt(r2);
	let y = sin(phi) * sqrt(r2);
	let z = sqrt(1 - r2);

	return vec3f(x, y, z);
}

fn lambertian_scattering_pdf(scattered : Ray) -> f32 {
	let cos_theta = max(0.0, dot(hitRec.global.normal, scattered.direction));
	return cos_theta / PI;
}

fn uniform_scattering_pdf(scattered : Ray) -> f32 {
	return 1 / (2 * PI);
}

var<private> unit_w : vec3f;
var<private> u : vec3f;
var<private> v : vec3f;
// creates an orthonormal basis
fn onb_build_from_w(w : vec3f) -> mat3x3f {
	unit_w = w;
	let a = select(vec3f(1, 0, 0), vec3f(0, 1, 0), abs(unit_w.x) > 0.9);
	v = normalize(cross(unit_w, a));
	u = cross(unit_w, v);

	return mat3x3f(u, v, unit_w);
}

fn onb_get_local(a : vec3f) -> vec3f {
	return u * a.x + v * a.y + unit_w * a.z;
}

fn onb_lambertian_scattering_pdf(scattered : Ray) -> f32 {
	let cosine_theta = dot(normalize(scattered.direction), unit_w);
	return max(0.0, cosine_theta/PI);
}

fn get_random_on_quad(q : Parallelogram, origin : vec3f) -> Ray {
	let p = q.Q + (rand_0_1() * q.u) + (rand_0_1() * q.v);
	return Ray(origin, normalize(p - origin));
}

fn get_random_on_quad_point(q : Parallelogram) -> vec3f {
	let p = q.Q + (rand_0_1() * q.u) + (rand_0_1() * q.v);
	return p;
}

fn light_pdf(ray : Ray, quad : Parallelogram) -> f32 {

	if(dot(ray.direction, quad.normal) > 0) {
		return MIN_FLOAT;
	}

	let denom = dot(quad.normal, ray.direction);

	if(abs(denom) < 1e-8) {
		return MIN_FLOAT;
	}

	let t = (quad.D - dot(quad.normal, ray.origin)) / denom;
	if(t <= 0.001 || t >= MAX_FLOAT) {
		return MIN_FLOAT;
	}

	let intersection = at(ray, t);
	let planar_hitpt_vector = intersection - quad.Q;
	let alpha = dot(quad.w, cross(planar_hitpt_vector, quad.v));
	let beta = dot(quad.w, cross(quad.u, planar_hitpt_vector));

	if(alpha < 0 || 1 < alpha || beta < 0 || 1 < beta) {
		return MIN_FLOAT;
	}

	var hitNormal = quad.normal;
	let front_face = dot(ray.direction, quad.normal) < 0;
	if(front_face == false)
	{
		hitNormal = -hitNormal;
	}

	let distance_squared = t * t * length(ray.direction) * length(ray.direction);
	let cosine = abs(dot(ray.direction, hitNormal) / length(ray.direction));

	return (distance_squared / (cosine * length(cross(lights.u, lights.v))));
}

//===================================================================
// final image output (aka resolve): tone mapping + gamma correction
//===================================================================

const full_screen_quad_positions = array<vec2f, 6>(
    vec2f(-1.0, -1.0),
    vec2f (1.0, -1.0),
    vec2f(-1.0,  1.0),

    vec2f(-1.0,  1.0),
    vec2f( 1.0, -1.0),
    vec2f( 1.0,  1.0),
);

// ACES approximation for tone mapping
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/:
fn aces_approx(v : vec3f) -> vec3f {
    let v1 = v * 0.6f; // TODO: WTF?
    const a = 2.51f;
    const b = 0.03f;
    const c = 2.43f;
    const d = 0.59f;
    const e = 0.14f;
    return clamp((v1*(a*v1+b))/(v1*(c*v1+d)+e), vec3(0.0f), vec3(1.0f));
}

@vertex fn vs(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4f {
    return vec4<f32>(full_screen_quad_positions[in_vertex_index], 0.0, 1.0);
}

@must_use
fn pixel_global_index(pixel_position: vec2f) -> u32 {
    return u32(pixel_position.y) * uniforms.frame_buffer_size.x + u32(pixel_position.x);
}

/*
Gradient noise from Jorge Jimenez's presentation:
Next Generation Post Processing in Call of Duty: Advanced Warfare
http://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare

Good post on usage: https://blog.frost.kiwi/GLSL-noise-and-radial-gradient/
*/
@must_use
fn gradient_noise(uv: vec2f) -> f32 {
	return fract(52.9829189 * fract(dot(uv, vec2f(0.06711056, 0.00583715))));
}

@must_use
fn pseudo_dither(color: vec3f, pixel_coordinate: vec2f) -> vec3f {
    return color + (1.0 / 255.0) * gradient_noise(pixel_coordinate) - (0.5 / 255.0);
}

@fragment fn fs(@builtin(position) fragment_coordinate: vec4f) -> @location(0) vec4f {
    let i = pixel_global_index(fragment_coordinate.xy);
    var color = pixel_color_buffer[i].xyz / uniforms.frame_number;

    color = aces_approx(color.xyz);
    color = pow(color.xyz, vec3f(1.0 / 2.2));
    color = pseudo_dither(color, fragment_coordinate.xy);

    return vec4f(color, 1.0);
}

//===================================================================
// deterministic ray tracing
//===================================================================

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_color_buffer_deterministic(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

    let camera = setup_camera();
	let pixel = make_common_color_evaluation_setup(pixel_index);

	let traced_color = path_trace_deterministic(camera, pixel);
	pixel_color_buffer[pixel_index] = vec4f(traced_color, 1.0);
}

@must_use
fn path_trace_deterministic(camera: Camera, pixel: Pixel) -> vec3f {
    if (uniforms.pixel_side_subdivision == 1) {
        return ray_color_deterministic(camera.origin, ray_and_differentials(camera, pixel, 0.5, 0.5));
    }

    var result_color = vec3f(0.0);
    let sub_pixel_step = 1.0 / f32(uniforms.pixel_side_subdivision - 1);
    for(var i = u32(0); i < uniforms.pixel_side_subdivision; i++) {
        for(var j = u32(0); j < uniforms.pixel_side_subdivision; j++) {
            let ray = ray_and_differentials(camera, pixel, sub_pixel_step * f32(i), sub_pixel_step * f32(j));
            result_color += ray_color_deterministic(camera.origin, ray);
        }
    }
    result_color /= f32(uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision);

    return result_color;
}

@must_use
fn ray_color_deterministic(camera_origin: vec3f, incident: RayAndDifferentials) -> vec3f {
    var accumulated_radiance = vec3f(0.0);

    var current_ray = incident.ray;
    var throughput = vec3f(1.0);
    for (var i = 0; i < DETERMINISTIC_MAX_RAY_BOUNCES; i++) {
        if (false == hit_scene(current_ray, MAX_FLOAT)) {
            accumulated_radiance += BACKGROUND_COLOR * throughput;
            break;
        }

        // TODO: differentials should be recalculated for each bounce; using same data for rays other than first ray (from eye) is incorrect
        let hit_material = hitMaterial;
        let hit_albedo = fetch_albedo(hitRec.local, current_ray.direction, hitRec.t, hit_material, incident.differentials);

        if (MATERIAL_LAMBERTIAN == hit_material.material_class) {
            accumulated_radiance += throughput * evaluate_dielectric_surface_color(camera_origin, hitRec, hit_material, hit_albedo);
            break;
        }

        if (MATERIAL_MIRROR == hit_material.material_class) {
            let reflected = evaluate_reflection(current_ray.direction, hitRec.global.normal, hitRec.global.position, hit_material.roughness);
            current_ray = Ray(hitRec.global.position + reflected * SECONDARY_RAY_START_BIAS, reflected);
            throughput *= hit_albedo;
        } else if (MATERIAL_GLASS == hit_material.material_class) {
            let stochastic = false;
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

@must_use
fn evaluate_dielectric_surface_color(camera_origin: vec3f, hit: HitRecord, hit_material: Material, hit_albedo: vec3f) -> vec3f {
    let light_center = lights.Q + (lights.u + lights.v) * 0.5;
    let to_light = light_center - hit.global.position;
    let to_light_distance = length(to_light);
    let to_light_direction = select(to_light / to_light_distance, vec3f(0.0), MIN_FLOAT > to_light_distance);
    let light_size = length(cross(lights.u, lights.v)) * DETERMINISTIC_SHADOW_LIGHT_SIZE_SCALE;

    let diffuse_fall_off = max(0.0, dot(hit.global.normal, to_light_direction));
    let to_camera_direction = normalize(camera_origin - hit.global.position);
    let reflected_light = reflect(-to_light_direction, hit.global.normal);
    let specular_fall_off = max(0.0, dot(reflected_light, to_camera_direction)) * diffuse_fall_off;

    //let shadow = evaluate_soft_shadow(hit.global.position, to_light_direction, light_size, DETERMINISTIC_SHADOW_START_BIAS, to_light_distance);
    let shadow = evaluate_hard_shadow(hit.global.position, to_light_direction, DETERMINISTIC_SHADOW_START_BIAS, to_light_distance);
    // shadow is in [0..1]: 0 is too dark -> lineary transform [0..1] into [K..1]
    let shadow_lightened = shadow * (1.0 - DETERMINISTIC_SHADOW_FLOOR) + DETERMINISTIC_SHADOW_FLOOR;
    let occlusion = approximate_ambient_occlusion(hit.global.position, hit.global.normal);

    let diffuse = diffuse_fall_off * hit_albedo * occlusion;
    let specular = specular_fall_off * hit_material.specular;
    let ambient = BACKGROUND_COLOR * hit_albedo * occlusion;
    let emissive = hit_material.emission;

    let light_color = materials[lights.material_id].emission;
    let reflected = mix(diffuse, specular, hit_material.specular_strength);

    return reflected * light_color * shadow_lightened + ambient + emissive;
}

@must_use
fn evaluate_reflection(incident: vec3f, normal: vec3f, hit_position: vec3f, roughness: f32) -> vec3f {
    let perfect = reflect(incident, normal);

    if (near_zero_scalar(roughness)) {
        return perfect;
    }

    let randomization = reflection_roughness_addition(hit_position, incident.z) * roughness;
    return normalize(perfect + randomization);
}

/* The randomization code below is from https://www.shadertoy.com/view/3sc3z4
The rand() function declared above gives not so random values — routhness
looks bad with it. */

@must_use
fn reflection_roughness_addition(position: vec3f, extra_seed: f32) -> vec3f {
    let phi = rand_from_seed((rand_0_1() + position.x + 0.357) * extra_seed) * 2.0 * PI;
	let theta = acos(2.0 * rand_from_seed((rand_0_1() + position.y + 16.35647) * extra_seed) - 1.0);
	let x = sin(theta) * cos(phi);
	let y = sin(theta) * sin(phi);
	let z = cos(theta);
	return vec3f(x, y, z);
}

@must_use
fn rand_from_seed(seed: f32) -> f32  {
    return fract(sin(seed) * 43758.5453123);
}

@must_use // 'to_light' expected to be normalized
fn evaluate_hard_shadow(position: vec3f, to_light: vec3f, min_ray_offset: f32, max_ray_offset: f32) -> f32 {
    if (hit_scene(Ray(position + to_light * min_ray_offset, to_light), max_ray_offset)) {
        if (any(hitMaterial.emission > vec3f(0.0))) {
            return 1.0;
        }
        return 0.0;
    }
    return 1.0;
}

@must_use // 'to_light' expected to be normalized
fn evaluate_soft_shadow(position: vec3f, to_light: vec3f, light_size: f32, min_ray_offset: f32, max_ray_offset: f32) -> f32 {
    var result: f32 = evaluate_hard_shadow(position, to_light, min_ray_offset, max_ray_offset); // or DETERMINISTIC_SHADOW_MARCHING_MAX
    var offset: f32 = min_ray_offset;
    var next_point = position + to_light * offset;
    for (var i = 0; i < DETERMINISTIC_SHADOW_RAY_MAX_STEPS; i++) {
        let signed_distance = sample_signed_distance(next_point, to_light);

        let candidate = signed_distance * light_size / offset;
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

@must_use
fn inside_aabb(box_min: vec3f, box_max: vec3f, probe: vec3f) -> bool {
    return all(probe >= box_min) && all(probe <= box_max);
}

@must_use // expected normalized 'direction'
fn sample_signed_distance(position: vec3f, direction: vec3f) -> f32 {
    var record = MAX_FLOAT;

    var node_index: i32 = 0;
    let max_index = i32(uniforms.bvh_length);
    while (node_index < max_index && NULL_POINTER_LINK != node_index) {
        let node = bvh_inflated[node_index];
        if(inside_aabb(node.aabb_min, node.aabb_max, position)) {
            if (PRIMITIVE_TYPE_SDF == node.primitive_type) {
                let sdf = sdf[node.primitive_index];
                let time = sdf_time[node.primitive_index];
                let candidate_distance = sample_signed_distance_function(sdf, position, direction, time);
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

@must_use // expected normalized 'direction'
fn sample_signed_distance_function(sdf: Sdf, position: vec3f, direction: vec3f, time: f32) -> f32 {
    let local_position = transform_point(sdf.inverse_location, position);
    let local_direction = normalize(transform_vector(to_mat3x3(sdf.inverse_location), direction));
    let local_distance = sample_sdf(sdf, local_position, time);
    let local_next = local_position + local_direction * local_distance;

    let global_next = transform_point(sdf.location, local_next);
    let global_offset = global_next - position;
    let global_distance = length(global_offset) * sign(dot(global_offset, direction));

    return global_distance;
}

@must_use // 'normal' is expected to be normalized
fn approximate_ambient_occlusion(posision: vec3f, normal: vec3f) -> f32 {
	var occlusion: f32 = 0.0;
    var fall_off: f32 = 1.0;
    for(var i = 0; i < DETERMINISTIC_AMBIENT_OCCLUSION_SAMPLES; i++) {
        let height = 0.01 + 0.12 * f32(i) / 4.0;
        let signed_distance = sample_signed_distance(posision + height * normal, normal);
        occlusion += max(0.0, (height - signed_distance) * fall_off);
        fall_off *= 0.95; // smaller values - lighter the shadow
        if(occlusion > 0.35) {
            break;
        }
    }

    // smaller values makes shadow outstand further from the caster; must be synced with aabb inflate ratio
    const OVERALL_INVERTED_SHADOW_LENGTH: f32 = 2.5;
    // bigger values makes shadow darker
    const SHADOW_DARKNESS: f32 = 7.0;

    return clamp(OVERALL_INVERTED_SHADOW_LENGTH - SHADOW_DARKNESS * occlusion, 0.0, 1.0);
}
