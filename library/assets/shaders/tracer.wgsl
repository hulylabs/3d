const PI = 3.1415926535897932385;

const MIN_FLOAT = 0.0001;
const MAX_FLOAT = 999999999.999;
const SECONDARY_RAY_START_BIAS = 0.0005;

const MATERIAL_LAMBERTIAN = 0.0;
const MATERIAL_MIRROR = 1.0;
const MATERIAL_GLASS = 2.0;
const MATERIAL_ISOTROPIC = 3.0;
const MATERIAL_ANISOTROPIC = 4.0;

const WORK_GROUP_SIZE_X = 8;
const WORK_GROUP_SIZE_Y = 8;
const WORK_GROUP_SIZE_Z = 1;
const WORK_GROUP_SIZE = vec3<u32>(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, WORK_GROUP_SIZE_Z);

const BACKGROUND_COLOR = vec3f(0.1);

const DETERMINISTIC_AMBIENT_OCCLUSION_SAMPLES = 5;
const DETERMINISTIC_SHADOW_RAY_MAX_STEPS = 256;
const DETERMINISTIC_SHADOW_MIN_STEP = 0.005;
const DETERMINISTIC_SHADOW_MAX_STEP = 0.50;
const DETERMINISTIC_SHADOW_START_BIAS = 0.02;
const DETERMINISTIC_SHADOW_LIGHT_SIZE_SCALE = 8.0; // bigger - crispier shadows
const DETERMINISTIC_SHADOW_MARCHING_MIN = -1.0;
const DETERMINISTIC_SHADOW_MARCHING_MAX = 1.0;
const DETERMINISTIC_SHADOW_FLOOR = 0.6;
const DETERMINISTIC_MAX_RAY_BOUNCES = 10;

const MONTE_CARLO_MAX_RAY_BOUNCES = 50;
const MONTE_CARLO_STRATIFY_SAMLING = false;
const MONTE_CARLO_IMPORTANCE_SAMPLING = true;
const STACK_SIZE = 20;
const MAX_SDF_RAY_MARCH_STEPS = 120;

@group(0) @binding( 0) var<uniform> uniforms : Uniforms;

@group(1) @binding( 0) var<storage, read_write> pixel_color_buffer: array<vec4f>;
@group(1) @binding( 1) var<storage, read_write> object_id_buffer: array<u32>;
@group(1) @binding( 2) var<storage, read_write> normal_buffer: array<vec4f>;
@group(1) @binding( 3) var<storage, read_write> albedo_buffer: array<vec4f>;

@group(2) @binding( 0) var<storage, read> quad_objs: array<Parallelogram>;
@group(2) @binding( 1) var<storage, read> sdf: array<Sdf>;
@group(2) @binding( 2) var<storage, read> triangles: array<Triangle>;
@group(2) @binding( 3) var<storage, read> materials: array<Material>;
@group(2) @binding( 4) var<storage, read> bvh: array<AABB>;

var<private> randState: u32 = 0u;
var<private> pixelCoords: vec2f;

var<private> hitRec: HitRecord;
var<private> scatterRec: ScatterRecord;
var<private> lights: Parallelogram;
var<private> ray_tmin: f32 = 0.000001;
var<private> ray_tmax: f32 = MAX_FLOAT;
var<private> stack: array<i32, STACK_SIZE>;

struct Uniforms {
	frame_buffer_size: vec2u,
	frame_buffer_area: u32,
	frame_buffer_aspect: f32, // width / height
	inverted_frame_buffer_size: vec2f,
	frame_number: f32,
	if_reset_frame_buffer: f32,
	view_matrix: mat4x4f,
	/* Consider a view ray defined by an origin (e.g., the eye position for a perspective camera)
    and a direction that intersects the view plane at a world-space pixel position.
    This matrix, when multiplied by the world-space pixel position, returns the ray's origin.
    For a perspective camera, the origin is always the eye position — the same for all pixels.
    For an orthographic camera, the origin lies on the camera plane and varies per pixel. */
	view_ray_origin_matrix : mat4x4f,

	parallelograms_count: u32,
	sdf_count: u32,
	triangles_count: u32,
	bvh_length: u32,
	pixel_side_subdivision: u32, // anti-aliasing level: bigger value -> slower render -> less jagged edges
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
	material_type: f32,
}

struct Parallelogram {
	Q: vec3f,
	u: vec3f,
	object_uid: u32,
	v: vec3f,
	D: f32,
	normal: vec3f,
	w: vec3f,
	material_id: f32,
}

struct Triangle {
	A : vec3f,
	B : vec3f,
	C : vec3f,
	normalA : vec3f,
	normalB : vec3f,
	object_uid : u32,
	normalC : vec3f,
	material_id : f32,
}

struct AABB {
	min : vec3f,
	right_offset : f32,

	max : vec3f,
	prim_type : f32,

	prim_id : f32,
	prim_count : f32,
	skip_link : f32,
	axis : f32,
}

struct Sdf {
    location : mat4x4f,
    inverse_location : mat4x4f,
    class_index : f32,
    material_id : f32,
    object_uid : u32,
}

struct HitRecord {
	p : vec3f,
	t : f32,
	normal : vec3f,
	front_face : bool,
	material : Material,
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

/// common

fn at(ray : Ray, t : f32) -> vec3f {
	return ray.origin + t * ray.direction;
}

// PCG prng
// https://www.shadertoy.com/view/XlGcRh
fn rand_0_1() -> f32 {
	randState = randState * 747796405u + 2891336453u;
	var word : u32 = ((randState >> ((randState >> 28u) + 4u)) ^ randState) * 277803737u;
	return f32((word >> 22u)^word) / 4294967295;
}

// random numbers from a normal distribution
fn randNormalDist() -> f32 {
	let theta = 2 * PI * rand_0_1();
	let rho = sqrt(-2 * log(rand_0_1()));
	return rho * cos(theta);
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
fn signed_distance_normal(point: vec3f, sdf: Sdf) -> vec3f {
    let e = vec2f(1.0,-1.0)*0.5773*0.0005;
    return normalize( e.xyy * sdf_select( sdf.class_index, point + e.xyy ) +
					  e.yyx * sdf_select( sdf.class_index, point + e.yyx ) +
					  e.yxy * sdf_select( sdf.class_index, point + e.yxy ) +
					  e.xxx * sdf_select( sdf.class_index, point + e.xxx ) );
}

@must_use
fn transform_point(transformation: mat4x4f, point: vec3f) -> vec3f {
    return (transformation * vec4f(point, 1.0f)).xyz;
}

@must_use
fn transform_vector(transformation: mat4x4f, vector: vec3f) -> vec3f {
    return (transformation * vec4f(vector, 0.0f)).xyz;
}

@must_use
fn transform_ray_parameter(transformation: mat4x4f, ray: Ray, parameter: f32, transformed_origin: vec3f) -> f32 {
    let point = transform_point(transformation, at(ray, parameter));
    return length(point - transformed_origin);
}

@must_use
fn hit_sdf(sdf: Sdf, tmin: f32, tmax: f32, ray: Ray) -> bool {
    let local_ray_origin = transform_point(sdf.inverse_location, ray.origin);
    let local_ray_direction = transform_vector(sdf.inverse_location, ray.direction);
    let local_ray = Ray(local_ray_origin, normalize(local_ray_direction));

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
        let signed_distance = sdf_select(sdf.class_index, candidate);
        let t_scaled = 0.0001 * local_t;
        if(abs(signed_distance) < t_scaled) {
            hitRec.p = transform_point(sdf.location, candidate);
            hitRec.t = length(hitRec.p - ray.origin);
            hitRec.normal = normalize(transform_vector(transpose(sdf.inverse_location), signed_distance_normal(candidate, sdf)));
            hitRec.front_face = sdf_select(sdf.class_index, local_ray.origin) >= 0;
            if(hitRec.front_face == false){
                hitRec.normal = -hitRec.normal;
            }
            hitRec.material = materials[i32(sdf.material_id)];
            return true;
        }
        let step_size = max(abs(signed_distance), t_scaled);
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

	// No hit if the ray is paraller to the plane
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
	hitRec.p = quad.Q + quad.v * beta + quad.u * alpha;
	hitRec.normal = quad.normal;
	hitRec.front_face = denom < 0.0;
	if(false == hitRec.front_face)
	{
		hitRec.normal = -hitRec.normal;
	}

	hitRec.material = materials[i32(quad.material_id)];
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
	let invDet = 1 / determinant;
	let dst = dot(ao, normal) * invDet;
	let u = dot(AC, dao) * invDet;
	let v = -dot(AB, dao) * invDet;
	let w = 1 - u - v;

	if(dst < tmin || dst > tmax || u < tmin || v < tmin || w < tmin) {
		return false;
	}

	hitRec.t = dst;
	hitRec.p = triangle.A * w + triangle.B * u + triangle.C * v;

	hitRec.normal = triangle.normalA * w + triangle.normalB * u + triangle.normalC * v;
	hitRec.normal = normalize(hitRec.normal);

	hitRec.front_face = dot(ray.direction, hitRec.normal) < 0;
	if(hitRec.front_face == false) {
		hitRec.normal = -hitRec.normal;
	}

	hitRec.material = materials[i32(triangle.material_id)];

	return true;
}

// https://medium.com/@bromanz/another-view-on-the-classic-ray-aabb-intersection-algorithm-for-bvh-traversal-41125138b525
fn hit_aabb(box_min : vec3f, box_max : vec3f, tmin : f32, tmax : f32, ray : Ray, invDir : vec3f) -> bool {
	var t0s = (box_min - ray.origin) * invDir;
	var t1s = (box_max - ray.origin) * invDir;

	var tsmaller = min(t0s, t1s);
	var tbigger = max(t0s, t1s);

	var t_min = max(tmin, max(tsmaller.x, max(tsmaller.y, tsmaller.z)));
	var t_max = min(tmax, min(tbigger.x, min(tbigger.y, tbigger.z)));

	return t_max > t_min;
}

fn get_lights() {
	for(var i = u32(0); i < uniforms.parallelograms_count; i++) {
		let emission = materials[i32(quad_objs[i].material_id)].emission;

		if(emission.x > 0.0) {
			lights = quad_objs[i];
			break;
		}
	}
}

// ACES approximation for tone mapping
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/):
fn aces_approx(v : vec3f) -> vec3f {
    let v1 = v * 0.6f;
    const a = 2.51f;
    const b = 0.03f;
    const c = 2.43f;
    const d = 0.59f;
    const e = 0.14f;
    return clamp((v1*(a*v1+b))/(v1*(c*v1+d)+e), vec3(0.0f), vec3(1.0f));
}

/// main

fn evaluate_pixel_index(
    global_invocation_id: vec3<u32>,
    num_workgroups: vec3<u32>,) -> u32 {
    let grid_dimension = WORK_GROUP_SIZE * num_workgroups;
    return
        global_invocation_id.z * (grid_dimension.x * grid_dimension.y) +
        global_invocation_id.y * (grid_dimension.x) +
        global_invocation_id.x ;
}

fn setup_pixel_coordinates(pixel_index: u32) {
    let x: u32 = pixel_index % uniforms.frame_buffer_size.x;
    let y: u32 = pixel_index / uniforms.frame_buffer_size.x;
    pixelCoords = vec2f(f32(x), f32(y));
}

fn setup_camera() {
    fovFactor = 1 / tan(60 * (PI / 180) / 2);
	cam_origin = uniforms.view_matrix[3].xyz;
}

fn pixel_outside_frame_buffer(pixel_index: u32) -> bool {
    return pixel_index >= uniforms.frame_buffer_area;
}

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_object_id_buffer(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

	setup_pixel_coordinates(pixel_index);
	setup_camera();

    let ray = ray_to_pixel(0.5, 0.5);
	let surface_intersection = trace_first_intersection(ray);
    object_id_buffer[pixel_index] = surface_intersection.object_uid;
    albedo_buffer[pixel_index] = vec4f(surface_intersection.albedo, 1.0f);
    normal_buffer[pixel_index] = vec4f(surface_intersection.normal, 0.0f);
}

fn make_common_color_evaluation_setup(pixel_index: u32) {
    setup_pixel_coordinates(pixel_index);
	setup_camera();
	get_lights();
}

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_color_buffer_monte_carlo(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

	make_common_color_evaluation_setup(pixel_index);

	randState = pixel_index + u32(uniforms.frame_number) * 719393;
	let traced_color = path_trace_monte_carlo();

	if(uniforms.if_reset_frame_buffer == 0) {
		pixel_color_buffer[pixel_index] = vec4f(pixel_color_buffer[pixel_index].xyz + traced_color, 1.0);
	} else {
	    pixel_color_buffer[pixel_index] = vec4f(traced_color, 1.0);
	}
}

/// shootRay

/*
x = aspect * (2 * (x / width) - 1) 	[ranges from -aspect to +aspect]
y = -(2 * (y / height) - 1)			[ranges from +1 to -1]
lower left pixel corner -> 0.5, 0.5 gives pixel's center;
*/
fn ray_to_pixel(sub_pixel_x: f32, sub_pixel_y: f32) -> Ray {
    let s = uniforms.frame_buffer_aspect * (2 * ((pixelCoords.x + sub_pixel_x) * uniforms.inverted_frame_buffer_size.x) - 1);
    let t = -1 * (2 * ((pixelCoords.y + sub_pixel_y) * uniforms.inverted_frame_buffer_size.y) - 1);
    return get_camera_ray(s, t);
}

@must_use
fn path_trace_monte_carlo() -> vec3f {
	let samples_count = uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision;
	var result_color = vec3f(0.0);
	if(MONTE_CARLO_STRATIFY_SAMLING) {
		let reciprocal_sqrt_samples_per_pixel = 1.0 / f32(uniforms.pixel_side_subdivision);
		for(var i = u32(0); i < uniforms.pixel_side_subdivision; i++) {
			for(var j = u32(0); j < uniforms.pixel_side_subdivision; j++) {
                let x = reciprocal_sqrt_samples_per_pixel * (f32(i) + rand_0_1());
                let y = reciprocal_sqrt_samples_per_pixel * (f32(j) + rand_0_1());
                let ray = ray_to_pixel(x, y);
				result_color += ray_color_monte_carlo(ray);
			}
		}
	} else {
		for(var i = u32(0); i < samples_count; i++) {
		    let ray = ray_to_pixel(rand_0_1(), rand_0_1());
			result_color += ray_color_monte_carlo(ray);
		}
	}
	result_color /= f32(samples_count);

	return result_color;
}

@must_use
fn trace_first_intersection(ray : Ray) -> FirstHitSurface {
    var closest_so_far = MAX_FLOAT;
    var hit_uid: u32 = 0;
    var hit_albedo: vec3f = vec3f(0.0f);
    var hit_normal: vec3f = vec3f(0.0f);

    for(var i = u32(0); i < uniforms.parallelograms_count; i++){
        let parallelogram = quad_objs[i];
        if(hit_quad(parallelogram, ray_tmin, closest_so_far, ray)) {
            hit_uid = parallelogram.object_uid;
            hit_albedo = materials[u32(parallelogram.material_id)].albedo;
            hit_normal = hitRec.normal;
            closest_so_far = hitRec.t;
        }
    }

    for(var i = u32(0); i < uniforms.sdf_count; i++){
        let sdf = sdf[i];
        if(hit_sdf(sdf, ray_tmin, closest_so_far, ray)){
            hit_uid = sdf.object_uid;
            hit_albedo = materials[u32(sdf.material_id)].albedo;
            hit_normal = hitRec.normal;
            closest_so_far = hitRec.t;
        }
    }

    const leafNode = 2;		// fix this hardcoding later
    var invDir = 1 / ray.direction;
    var toVisitOffset = 0;
    var curNodeIdx = 0;
    var node = bvh[curNodeIdx];

    while(true) {
        node = bvh[curNodeIdx];

        if(hit_aabb(node.min, node.max, ray_tmin, closest_so_far, ray, invDir)) {
            if(i32(node.prim_type) == leafNode) {
                let startPrim = i32(node.prim_id);
                let countPrim = i32(node.prim_count);
                for(var j = 0; j < countPrim; j++) {
                    let triangle = triangles[startPrim + j];
                    if(hit_triangle(triangle, ray_tmin, closest_so_far, ray)) {
                        hit_uid = triangle.object_uid;
                        hit_albedo = materials[u32(triangle.material_id)].albedo;
                        hit_normal = hitRec.normal;
                        closest_so_far = hitRec.t;
                    }
                }

                if(toVisitOffset == 0){
                    break;
                }
                toVisitOffset--;
                curNodeIdx = stack[toVisitOffset];
            } else {
                if(ray.direction[i32(node.axis)] < 0) {
                    stack[toVisitOffset] = curNodeIdx + 1;
                    toVisitOffset++;
                    curNodeIdx = i32(node.right_offset);
                } else {
                    stack[toVisitOffset] = i32(node.right_offset);
                    toVisitOffset++;
                    curNodeIdx++;
                }
            }
        } else {
            if(toVisitOffset == 0) {
                break;
            }

            toVisitOffset--;
            curNodeIdx = stack[toVisitOffset];
        }

        if(toVisitOffset >= STACK_SIZE) {
            break;
        }
    }
    return FirstHitSurface(hit_uid, hit_albedo, hit_normal);
}

var<private> fovFactor : f32;
var<private> cam_origin : vec3f;

fn get_camera_ray(s : f32, t : f32) -> Ray {
	let eye_to_pixel_direction = (uniforms.view_matrix * vec4f(vec3f(s, t, -fovFactor), 0.0)).xyz;
	let pixel_world_space = vec4(cam_origin + eye_to_pixel_direction, 1.0);

	let ray_origin_world_space = (uniforms.view_ray_origin_matrix * pixel_world_space).xyz;
	let direction = normalize(pixel_world_space.xyz - ray_origin_world_space);

	return Ray(ray_origin_world_space, direction);
}

/// hitRay

fn hit_scene(ray : Ray) -> bool
{
	var closest_so_far = MAX_FLOAT;
	var hit_anything = false;

	for(var i = u32(0); i < uniforms.parallelograms_count; i++) {
		if(hit_quad(quad_objs[i], ray_tmin, closest_so_far, ray)) {
			hit_anything = true;
			closest_so_far = hitRec.t;
		}
	}

	// traversing BVH using a stack implementation
	// https://pbr-book.org/3ed-2018/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies#CompactBVHForTraversal

	const leafNode = 2;		// fix this hardcoding later
	var invDir = 1 / ray.direction;
	var toVisitOffset = 0;
	var curNodeIdx = 0;
	var node = bvh[curNodeIdx];

	while(true) {
		node = bvh[curNodeIdx];

		if(hit_aabb(node.min, node.max, ray_tmin, closest_so_far, ray, invDir)) {
			if(i32(node.prim_type) == leafNode) {
				let startPrim = i32(node.prim_id);
				let countPrim = i32(node.prim_count);
				for(var j = 0; j < countPrim; j++) {
					if(hit_triangle(triangles[startPrim + j], ray_tmin, closest_so_far, ray))
					{
						hit_anything = true;
						closest_so_far = hitRec.t;
					}
				}

				if(toVisitOffset == 0) {
					break;
				}
				toVisitOffset--;
				curNodeIdx = stack[toVisitOffset];
			} else {
				if(ray.direction[i32(node.axis)] < 0) {
					stack[toVisitOffset] = curNodeIdx + 1;
					toVisitOffset++;
					curNodeIdx = i32(node.right_offset);
				} else {
					stack[toVisitOffset] = i32(node.right_offset);
					toVisitOffset++;
					curNodeIdx++;
				}
			}
		} else {
			if(toVisitOffset == 0) {
				break;
			}

			toVisitOffset--;
			curNodeIdx = stack[toVisitOffset];
		}

		if(toVisitOffset >= STACK_SIZE) {
			break;
		}
	}

	for(var i = u32(0); i < uniforms.sdf_count; i++) {
        if(hit_sdf(sdf[i], ray_tmin, closest_so_far, ray)) {
            hit_anything = true;
            closest_so_far = hitRec.t;
        }
    }

	return hit_anything;
}

// ============== Other BVH traversal methods (brute force and using skip pointers) =================

// fn hit_skipPointers(ray : Ray) -> bool
// {
// 	var closest_so_far = MAX_FLOAT;
// 	var hit_anything = false;

// 	var invDir = 1 / ray.direction;
// 	var i = 0;
// 	while(i < uniforms.bvh_length && i != -1)
// 	{
// 		if(hit_aabb(bvh[i], ray_tmin, closest_so_far, ray, invDir))
// 		{

// 			let t = i32(bvh[i].prim_type);

// 			if(t == 2) {

// 				let startPrim = i32(bvh[i].prim_id);
// 				let countPrim = i32(bvh[i].prim_count);
// 				for(var j = 0; j < countPrim; j++)
// 				{
// 					if(hit_triangle(triangles[startPrim + j], ray_tmin, closest_so_far, ray))
// 					{
// 						hit_anything = true;
// 						closest_so_far = hitRec.t;
// 					}
// 				}
// 			}

// 			i++;
// 		}

// 		else
// 		{
// 			i = i32(bvh[i].skip_link);
// 		}
// 	}

// 	for(var i = 0; i < uniforms.parallelograms_count; i++)
// 	{
// 		if(hit_quad(quad_objs[i], ray_tmin, closest_so_far, ray))
// 		{
// 			hit_anything = true;
// 			closest_so_far = hitRec.t;
// 		}
// 	}

// 	return hit_anything;
// }

/// traceRay

// https://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Path_Tracing#Implementation

fn ray_color_monte_carlo(incident_ray : Ray) -> vec3f {

	var current_ray = incident_ray;
	var accumulated_radiance = vec3f(0.0);	// initial radiance (pixel color) is black
	var throughput = vec3f(1.0);		// initial throughput is 1 (no attenuation)

	for(var i = 0; i < MONTE_CARLO_MAX_RAY_BOUNCES; i++) {
		if(hit_scene(current_ray) == false) {
			accumulated_radiance += (BACKGROUND_COLOR * throughput);
			break;
		}

		var emission_color = hitRec.material.emission;
		if(!hitRec.front_face) {
			emission_color = vec3f(0.0);
		}

		if(MONTE_CARLO_IMPORTANCE_SAMPLING) {
			let scatterred_surface = material_scatter(current_ray);

			if(scatterRec.skip_pdf) {
				accumulated_radiance += emission_color * throughput;
				throughput *= mix(hitRec.material.albedo, hitRec.material.specular, doSpecular);

				current_ray = scatterRec.skip_pdf_ray;
				current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
				continue;
			}

            const LIGHT_SAMPLING_PROBABILITY = 0.2;
			var scattered: Ray;
			if(rand_0_1() > LIGHT_SAMPLING_PROBABILITY) {
				scattered = scatterred_surface;
			} else {
			    scattered = get_random_on_quad(lights, hitRec.p);
			}

			let lambertian_pdf = onb_lambertian_scattering_pdf(scattered);
			let light_pdf = light_pdf(scattered, lights);
			let pdf = LIGHT_SAMPLING_PROBABILITY * light_pdf + (1.0 - LIGHT_SAMPLING_PROBABILITY) * lambertian_pdf;

			if(pdf <= 0.00001) {
				return emission_color * throughput;
			}

			accumulated_radiance += emission_color * throughput;
			throughput *= ((lambertian_pdf * mix(hitRec.material.albedo, hitRec.material.specular, doSpecular)) / pdf);
			current_ray = scattered;
			current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
		} else {
			let scattered = material_scatter(current_ray);

			accumulated_radiance += emission_color * throughput;
			throughput *= mix(hitRec.material.albedo, hitRec.material.specular, doSpecular);

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

/// scatterRay

var<private> doSpecular : f32;
fn material_scatter(ray_in : Ray) -> Ray {
	var scattered = Ray(vec3f(0.0), vec3f(0.0));
	doSpecular = 0;
	if(hitRec.material.material_type == MATERIAL_LAMBERTIAN) {

		let uvw = onb_build_from_w(hitRec.normal);
		var diffuse_dir = cosine_sampling_wrt_Z();
		diffuse_dir = normalize(onb_get_local(diffuse_dir));

		scattered = Ray(hitRec.p, diffuse_dir);

		doSpecular = select(0.0, 1.0, rand_0_1() < hitRec.material.specular_strength);

		var specular_dir = reflect(ray_in.direction, hitRec.normal);
		specular_dir = normalize(mix(specular_dir, diffuse_dir, hitRec.material.roughness));

		scattered = Ray(hitRec.p, normalize(mix(diffuse_dir, specular_dir, doSpecular)));

		scatterRec.skip_pdf = false;

		if(doSpecular == 1.0) {
			scatterRec.skip_pdf = true;
			scatterRec.skip_pdf_ray = scattered;
		}
	}
	else if(hitRec.material.material_type == MATERIAL_MIRROR) {
		var reflected = reflect(ray_in.direction, hitRec.normal);
		scattered = Ray(hitRec.p, normalize(reflected + hitRec.material.roughness * uniform_random_in_unit_sphere()));

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}
	else if(hitRec.material.material_type == MATERIAL_GLASS) {
		let stochastic = true;
		scattered = glass_scatter(hitRec, ray_in.direction, stochastic);

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}
	else if(hitRec.material.material_type == MATERIAL_ISOTROPIC) {
		let g = hitRec.material.specular_strength;
		let cos_hg = (1 + g*g - pow(((1 - g*g) / (1 - g + 2*g*rand_0_1())), 2.0)) / (2 * g);
		let sin_hg = sqrt(1 - cos_hg * cos_hg);
		let phi = 2 * PI * rand_0_1();

		let hg_dir = vec3f(sin_hg * cos(phi), sin_hg * sin(phi), cos_hg);

		let uvw = onb_build_from_w(ray_in.direction);
		scattered = Ray(hitRec.p, normalize(onb_get_local(hg_dir)));

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}

	return scattered;
}

@must_use
fn glass_scatter(hit: HitRecord, in_ray_direction: vec3f, stochastic: bool) -> Ray {
    var ir = hit.material.refractive_index_eta;
    if(hit.front_face) {
        ir = (1.0 / ir);
    }

    let unit_direction = in_ray_direction;
    let cos_theta = min(-dot(unit_direction, hit.normal), 1.0);
    let sin_theta = sqrt(1 - cos_theta*cos_theta);

    var direction = vec3f(0.0);
    if(ir * sin_theta > 1.0) {
        direction = reflect(unit_direction, hit.normal);
    } else {
        if (stochastic) {
            if (reflectance(cos_theta, ir) > rand_0_1()) {
                direction = reflect(unit_direction, hit.normal);
            } else {
                direction = refract(unit_direction, hit.normal, ir);
            }
        } else {
            direction = refract(unit_direction, hit.normal, ir);
        }
    }

    if(near_zero(direction)) {
        direction = hit.normal;
    }

    return Ray(hit.p, direction);
}

/// importanceSampling

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
	let sign_dot = select(1.0, 0.0, dot(on_unit_sphere, hitRec.normal) > 0.0);
    return normalize(mix(on_unit_sphere, -on_unit_sphere, sign_dot));
}

fn cosine_sampling_hemisphere() -> vec3f {
	return uniform_random_in_unit_sphere() + hitRec.normal;
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
	let cos_theta = max(0.0, dot(hitRec.normal, scattered.direction));
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
/// final image output (aka resolve): tone mapping + gamma correction
//===================================================================

const full_screen_quad_positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32> (1.0, -1.0),
    vec2<f32>(-1.0,  1.0),

    vec2<f32>(-1.0,  1.0),
    vec2<f32>( 1.0, -1.0),
    vec2<f32>( 1.0,  1.0),
);

@vertex fn vs(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4f {
    return vec4<f32>(full_screen_quad_positions[in_vertex_index], 0.0, 1.0);
}

@must_use
fn pixel_global_index(pixel_position: vec2f) -> u32 {
    return u32(pixel_position.y) * uniforms.frame_buffer_size.x + u32(pixel_position.x);
}

@fragment fn fs(@builtin(position) fragment_coordinate: vec4f) -> @location(0) vec4f {
    let i = pixel_global_index(fragment_coordinate.xy);
    var color = pixel_color_buffer[i].xyz / uniforms.frame_number;

    color = aces_approx(color.xyz);
    color = pow(color.xyz, vec3f(1.0/2.2));

    if(1 == uniforms.if_reset_frame_buffer) {
        pixel_color_buffer[i] = vec4f(0.0);
    }

    return vec4f(color, 1);
}

//===================================================================
/// deterministic ray tracing
//===================================================================

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_color_buffer_deterministic(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

	make_common_color_evaluation_setup(pixel_index);

	let traced_color = path_trace_deterministic();
	pixel_color_buffer[pixel_index] = vec4f(traced_color, 1.0);
}

@must_use
fn path_trace_deterministic() -> vec3f {
    if (uniforms.pixel_side_subdivision == 1) {
        return ray_color_deterministic(ray_to_pixel(0.5, 0.5));
    }

    var result_color = vec3f(0.0);
    let sub_pixel_step = 1.0 / f32(uniforms.pixel_side_subdivision - 1);
    for(var i = u32(0); i < uniforms.pixel_side_subdivision; i++) {
        for(var j = u32(0); j < uniforms.pixel_side_subdivision; j++) {
            let ray = ray_to_pixel(sub_pixel_step * f32(i), sub_pixel_step * f32(j));
            result_color += ray_color_deterministic(ray);
        }
    }
    result_color /= f32(uniforms.pixel_side_subdivision * uniforms.pixel_side_subdivision);

    return result_color;
}

@must_use
fn ray_color_deterministic(incident_ray: Ray) -> vec3f {
    var accumulated_radiance = vec3f(0.0);

    var current_ray = incident_ray;
    var throughput = vec3f(1.0);
    for (var i = 0; i < DETERMINISTIC_MAX_RAY_BOUNCES; i++) {
        if (false == hit_scene(current_ray)) {
            accumulated_radiance += BACKGROUND_COLOR * throughput;
            break;
        }

        if (MATERIAL_LAMBERTIAN == hitRec.material.material_type) {
            accumulated_radiance += throughput * evaluate_dielectric_surface_color(hitRec);
            break;
        }

        if (MATERIAL_MIRROR == hitRec.material.material_type) {
            let reflected = evaluate_reflection(current_ray.direction, hitRec.normal, hitRec.p, hitRec.material.roughness);
            current_ray = Ray(hitRec.p + reflected * SECONDARY_RAY_START_BIAS, reflected);
            throughput *= hitRec.material.albedo;
        } else if (MATERIAL_GLASS == hitRec.material.material_type) {
            let stochastic = false;
            current_ray = glass_scatter(hitRec, current_ray.direction, stochastic);
            current_ray.origin += current_ray.direction * SECONDARY_RAY_START_BIAS;
            throughput *= hitRec.material.albedo;
        } else {
            accumulated_radiance += hitRec.material.albedo;
            break;
        }
    }

    return accumulated_radiance;
}

@must_use
fn evaluate_dielectric_surface_color(hit: HitRecord) -> vec3f {
    let light_center = lights.Q + (lights.u + lights.v) * 0.5;
    let to_light = light_center - hit.p;
    let to_light_direction = normalize(to_light);
    let light_size = length(cross(lights.u, lights.v)) * DETERMINISTIC_SHADOW_LIGHT_SIZE_SCALE;

    let diffuse_fall_off = max(0.0, dot(hit.normal, to_light_direction));
    let to_camera_direction = normalize(cam_origin - hit.p);
    let reflected_light = reflect(-to_light_direction, hit.normal);
    let specular_fall_off = max(0.0, dot(reflected_light, to_camera_direction)) * diffuse_fall_off;

    let shadow = shadow(hit.p, to_light_direction, light_size, DETERMINISTIC_SHADOW_START_BIAS, 1.0);
    // shadow is in [0..1]: 0 is too dark -> lineary transform [0..1] into [K..1]
    let shadow_lightened = shadow * (1.0 - DETERMINISTIC_SHADOW_FLOOR) + DETERMINISTIC_SHADOW_FLOOR;
    let occlusion = approximate_ambient_occlusion(hit.p, hit.normal);

    let diffuse = diffuse_fall_off * hit.material.albedo * occlusion;
    let specular = specular_fall_off * hit.material.specular;
    let ambient = BACKGROUND_COLOR * hit.material.albedo * occlusion;
    let emissive = hit.material.emission;

    let light_color = materials[u32(lights.material_id)].emission;
    let reflected = mix(diffuse, specular, hit.material.specular_strength);

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
fn shadow(position: vec3f, to_light: vec3f, light_size: f32, min_ray_offset: f32, max_ray_offset: f32) -> f32 {
    /* https://iquilezles.org/articles/rmshadows/ - the algorithm.
    Main idea: account not only outer penumbra (ray close to object),
    but also "inner penumbra" (ray barely inside the object). */

    var result: f32 = DETERMINISTIC_SHADOW_MARCHING_MAX;
    var offset: f32 = min_ray_offset;
    var next_point = position + to_light * offset;
    for (var i = 0; i < DETERMINISTIC_SHADOW_RAY_MAX_STEPS; i++) {
        let step = sample_signed_distance(next_point, to_light);

        result = min(result, step.signed_distance * light_size / offset);
        if(result < DETERMINISTIC_SHADOW_MARCHING_MIN) {
            break;
        }

        offset += clamp(step.signed_distance, DETERMINISTIC_SHADOW_MIN_STEP, DETERMINISTIC_SHADOW_MAX_STEP);
        if(offset > max_ray_offset) {
            break;
        }

        next_point = position + to_light * offset;
    }
    result = max(result, DETERMINISTIC_SHADOW_MARCHING_MIN);
    return smoothstep(DETERMINISTIC_SHADOW_MARCHING_MIN, DETERMINISTIC_SHADOW_MARCHING_MAX, result);
}

struct RayMarchStep {
    new_position: vec3f,
    signed_distance: f32,
}

@must_use // expected normalized 'direction'
fn sample_signed_distance(position: vec3f, direction: vec3f) -> RayMarchStep {
    var record: RayMarchStep = RayMarchStep(position, MAX_FLOAT);
    for (var i = u32(0); i < uniforms.sdf_count; i++) {
        let sdf = sdf[i];

        let local_position = transform_point(sdf.inverse_location, position);
        let local_direction = normalize(transform_vector(sdf.inverse_location, direction));
        let local_distance = sdf_select(sdf.class_index, local_position);
        let local_next = local_position + local_direction * local_distance;

        let global_next = transform_point(sdf.location, local_next);
        let global_offset = global_next - position;
        let global_distance = length(global_offset) * sign(dot(global_offset, direction));

        if (global_distance < record.signed_distance) {
            record = RayMarchStep(global_next, global_distance);
        }
    }
    return record;
}

@must_use // 'normal' is expected to be normalized
fn approximate_ambient_occlusion(posision: vec3f, normal: vec3f) -> f32 {
	var occlusion: f32 = 0.0;
    var fall_off: f32 = 1.0;
    for(var i = 0; i < DETERMINISTIC_AMBIENT_OCCLUSION_SAMPLES; i++) {
        let height = 0.01 + 0.12 * f32(i) / 4.0;
        let step = sample_signed_distance(posision + height * normal, normal);
        occlusion += (height - step.signed_distance) * fall_off;
        fall_off *= 0.95;
        if(occlusion > 0.35) {
            break;
        }
    }
    return clamp(1.0 - 3.0 * occlusion, 0.0, 1.0) * (0.5 + 0.5 * normal.y);
}
