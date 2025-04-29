/// header

const PI = 3.1415926535897932385;

const MIN_FLOAT = 0.0001;
const MAX_FLOAT = 999999999.999;

const LAMBERTIAN = 0.0;
const MIRROR = 1.0;
const GLASS = 2.0;
const ISOTROPIC = 3.0;
const ANISOTROPIC = 4.0;

const WORK_GROUP_SIZE_X = 8;
const WORK_GROUP_SIZE_Y = 8;
const WORK_GROUP_SIZE_Z = 1;
const WORK_GROUP_SIZE = vec3<u32>(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, WORK_GROUP_SIZE_Z);

const NUM_SAMPLES = 2.0;
const MAX_BOUNCES = 50;
const STRATIFY = true;
const IMPORTANCE_SAMPLING = true;
const STACK_SIZE = 20;
const MAX_SDF_RAY_MARCH_STEPS = 20;

@group(0) @binding( 0) var<uniform> uniforms : Uniforms;

@group(1) @binding( 0) var<storage, read_write> pixel_color_buffer: array<vec4f>;
@group(1) @binding( 1) var<storage, read_write> object_id_buffer: array<u32>;
@group(1) @binding( 2) var<storage, read_write> normal_buffer: array<vec4f>;
@group(1) @binding( 3) var<storage, read_write> albedo_buffer: array<vec4f>;

@group(2) @binding( 0) var<storage, read> sphere_objs: array<Sphere>;
@group(2) @binding( 1) var<storage, read> quad_objs: array<Parallelogram>;
@group(2) @binding( 2) var<storage, read> sdf: array<SdfBox>;
@group(2) @binding( 3) var<storage, read> triangles: array<Triangle>;
@group(2) @binding( 4) var<storage, read> materials: array<Material>;
@group(2) @binding( 5) var<storage, read> bvh: array<AABB>;

var<private> randState : u32 = 0u;
var<private> pixelCoords : vec3f;

var<private> hitRec : HitRecord;
var<private> scatterRec : ScatterRecord;
var<private> lights : Parallelogram;
var<private> ray_tmin : f32 = 0.000001;
var<private> ray_tmax : f32 = MAX_FLOAT;
var<private> stack : array<i32, STACK_SIZE>;

struct Uniforms {
	frame_buffer_size : vec2f,
	frame_number : f32,
	if_reset_frame_buffer : f32,
	view_matrix : mat4x4f,
	/* Consider a view ray defined by an origin (e.g., the eye position for a perspective camera)
    and a direction that intersects the view plane at a world-space pixel position.
    This matrix, when multiplied by the world-space pixel position, returns the ray's origin.
    For a perspective camera, the origin is always the eye position — the same for all pixels.
    For an orthographic camera, the origin lies on the camera plane and varies per pixel. */
	view_ray_origin_matrix : mat4x4f,

	spheres_count: u32,
	parallelograms_count: u32,
	sdf_count: u32,
	triangles_count: u32,
	bvh_length: u32,
}

struct Ray {
	origin : vec3f,
	dir : vec3f,
}

struct Material {
	color : vec3f,			// diffuse color
	specularColor : vec3f,	// specular color
	emissionColor : vec3f,	// emissive color
	specularStrength : f32,	// chance that a ray hitting would reflect specularly
	roughness : f32,		// diffuse strength
	eta : f32,				// refractive index
	material_type : f32,
}

struct Sphere {
	center : vec3f,
	r : f32,
	object_uid : u32,
	material_id : f32,
}

struct Parallelogram {
	Q : vec3f,
	u : vec3f,
	object_uid : u32,
	v : vec3f,
	D : f32,
	normal : vec3f,
	w : vec3f,
	material_id : f32,
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

struct SdfBox {
    location : mat4x4f,
    inverse_location : mat4x4f,
    half_size : vec3f,
    corners_radius : f32,
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
	return ray.origin + t * ray.dir;
}

// PCG prng
// https://www.shadertoy.com/view/XlGcRh
fn rand2D() -> f32
{
	randState = randState * 747796405u + 2891336453u;
	var word : u32 = ((randState >> ((randState >> 28u) + 4u)) ^ randState) * 277803737u;
	return f32((word >> 22u)^word) / 4294967295;
}

// random numbers from a normal distribution
fn randNormalDist() -> f32 {
	let theta = 2 * PI * rand2D();
	let rho = sqrt(-2 * log(rand2D()));
	return rho * cos(theta);
}

fn random_double(min : f32, max : f32) -> f32 {
	return min + (max - min) * rand2D();
}

fn near_zero(v : vec3f) -> bool {
	return (abs(v[0]) < 0 && abs(v[1]) < 0 && abs(v[2]) < 0);
}

fn signed_distance_to_box(point: vec3f, box: SdfBox) -> f32 {
    let d = abs(point) - box.half_size + box.corners_radius;
    return min(max(d.x, max(d.y, d.z)), 0.0) + length(max(d, vec3f(0.0))) - box.corners_radius;
}

fn signed_distance_normal(point: vec3f, box: SdfBox) -> vec3f {
    let e = vec2f(1.0,-1.0)*0.5773*0.0005;
    return normalize( e.xyy * signed_distance_to_box( point + e.xyy, box ) +
					  e.yyx * signed_distance_to_box( point + e.yyx, box ) +
					  e.yxy * signed_distance_to_box( point + e.yxy, box ) +
					  e.xxx * signed_distance_to_box( point + e.xxx, box ) );
}

fn hit_sdf(sdf : SdfBox, tmin : f32, tmax : f32, ray : Ray) -> bool {
    var t = tmin;
    var i: i32 = 0;
    let local_ray = Ray( (sdf.inverse_location * vec4f(ray.origin, 1.0f)).xyz , (sdf.inverse_location * vec4f(ray.dir, 0.0f)).xyz );
    let outside = signed_distance_to_box(local_ray.origin, sdf) >= 0;
    loop {
        if (i >= MAX_SDF_RAY_MARCH_STEPS) {
            break;
        }
        if (t>tmax) {
            break;
        }
        let candidate = at(local_ray, t);
        let signed_distance = signed_distance_to_box(candidate, sdf);
        let t_scaled = 0.0001 * t;
        if(abs(signed_distance)<t_scaled) {
            hitRec.t = t;
            hitRec.p = at(ray, t);
            hitRec.normal = (sdf.location * vec4f(signed_distance_normal(candidate, sdf), 0.0f)).xyz; // assume transform is orthogonal
            hitRec.front_face = outside;
            hitRec.material = materials[i32(sdf.material_id)];
            return true;
        }
        let step_size = max(abs(signed_distance), t_scaled);
        t += step_size;
        i = i + 1;
    }
    return false;
}

fn hit_sphere(sphere : Sphere, tmin : f32, tmax : f32, ray : Ray) -> bool {
	let oc = ray.origin - sphere.center;
	let a = dot(ray.dir, ray.dir);
	let half_b = dot(ray.dir, oc);
	let c = dot(oc, oc) - sphere.r * sphere.r;
	let discriminant = half_b*half_b - a*c;

	if(discriminant < 0) {
		return false;
	}

	let sqrtd = sqrt(discriminant);
	var root = (-half_b - sqrtd) / a;
	if(root <= tmin || root >= tmax)
	{
		root = (-half_b + sqrtd) / a;
		if(root <= tmin || root >= tmax)
		{
			return false;
		}
	}

	hitRec.t = root;
	hitRec.p = at(ray, root);

	hitRec.normal = normalize(hitRec.p - sphere.center);

	hitRec.front_face = dot(ray.dir, hitRec.normal) < 0;
	if(hitRec.front_face == false)
	{
		hitRec.normal = -hitRec.normal;
	}

	hitRec.material = materials[i32(sphere.material_id)];
	return true;
}

fn hit_sphere_local(sphere : Sphere, tmin : f32, tmax : f32, ray : Ray) -> f32 {
	let oc = ray.origin - sphere.center;
	let a = dot(ray.dir, ray.dir);
	let half_b = dot(ray.dir, oc);
	let c = dot(oc, oc) - sphere.r * sphere.r;
	let discriminant = half_b*half_b - a*c;

	if(discriminant < 0) {
		return MAX_FLOAT + 1;
	}

	let sqrtd = sqrt(discriminant);
	var root = (-half_b - sqrtd) / a;
	if(root <= tmin || root >= tmax)
	{
		root = (-half_b + sqrtd) / a;
		if(root <= tmin || root >= tmax)
		{
			return MAX_FLOAT + 1;
		}
	}

	return root;
}

fn hit_volume(sphere : Sphere, tmin : f32, tmax : f32, ray : Ray) -> bool {

	var rec1 = hit_sphere_local(sphere, -MAX_FLOAT, MAX_FLOAT, ray);
	if(rec1 == MAX_FLOAT + 1) {
		return false;
	}

	var rec2 = hit_sphere_local(sphere, rec1 + 0.0001, MAX_FLOAT, ray);
	if(rec2 == MAX_FLOAT + 1) {
		return false;
	}

	if(rec1 < tmin) {
		rec1 = tmin;
	}

	if(rec2 > tmax) {
		rec2 = tmax;
	}

	if(rec1 >= rec2) {
		return false;
	}

	if(rec1 < 0) {
		rec1 = 0;
	}

	hitRec.material = materials[i32(sphere.material_id)];

	let ray_length = length(ray.dir);
	let dist_inside = (rec2 - rec1) * ray_length;
	let hit_dist = hitRec.material.roughness * log(rand2D());

	if(hit_dist > dist_inside) {
		return false;
	}

	hitRec.t = rec1 + (hit_dist / ray_length);
	hitRec.p = at(ray, hitRec.t);
	hitRec.normal = normalize(hitRec.p - sphere.center);
	hitRec.front_face = true;

	return true;
}

fn hit_quad(quad : Parallelogram, tmin : f32, tmax : f32, ray : Ray) -> bool {

	if(dot(ray.dir, quad.normal) > 0) {
		return false;
	}

	let denom = dot(quad.normal, ray.dir);

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
	hitRec.p = intersection;
	hitRec.normal = quad.normal;
	hitRec.front_face = dot(ray.dir, hitRec.normal) < 0;
	if(hitRec.front_face == false)
	{
		hitRec.normal = -hitRec.normal;
	}

	hitRec.material = materials[i32(quad.material_id)];
	return true;
}

// https://stackoverflow.com/questions/42740765/
// https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection.html
fn hit_triangle(tri : Triangle, tmin : f32, tmax : f32, incidentRay : Ray) -> bool {

	let ray = Ray(incidentRay.origin, incidentRay.dir);

	let AB = tri.B - tri.A;
	let AC = tri.C - tri.A;
	let normal = cross(AB, AC);
	let determinant = -dot(ray.dir, normal);

	// CULLING
	if(abs(determinant) < tmin) {
		return false;
	}

	let ao = ray.origin - tri.A;
	let dao = cross(ao, ray.dir);

	// calculate dist to triangle & barycentric coordinates of intersection point
	let invDet = 1 / determinant;
	let dst = dot(ao, normal) * invDet;
	let u = dot(AC, dao) * invDet;
	let v = -dot(AB, dao) * invDet;
	let w = 1 - u - v;

	if(dst < tmin || dst > tmax || u < tmin || v < tmin || w < tmin)
	{
		return false;
	}

	hitRec.t = dst;
	hitRec.p = at(incidentRay, dst);

	hitRec.normal = tri.normalA * w + tri.normalB * u + tri.normalC * v;
	hitRec.normal = normalize(hitRec.normal);

	hitRec.front_face = dot(incidentRay.dir, hitRec.normal) < 0;
	if(hitRec.front_face == false)
	{
		hitRec.normal = -hitRec.normal;
	}

	hitRec.material = materials[i32(tri.material_id)];

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

fn get_lights() -> bool {
	for(var i = u32(0); i < uniforms.parallelograms_count; i++) {
		let emission = materials[i32(quad_objs[i].material_id)].emissionColor;

		if(emission.x > 0.0) {
			lights = quad_objs[i];
			break;
		}
	}

	return true;
}

// ACES approximation for tone mapping
// https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/):
fn aces_approx(v : vec3f) -> vec3f
{
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
    pixelCoords = vec3f(f32(pixel_index) % uniforms.frame_buffer_size.x, f32(pixel_index) / uniforms.frame_buffer_size.x, 1);
}

fn setup_camera() {
    fovFactor = 1 / tan(60 * (PI / 180) / 2);
	cam_origin = uniforms.view_matrix[3].xyz;
}

fn pixel_outside_frame_buffer(pixel_index: u32) -> bool {
    return f32(pixel_index) >= uniforms.frame_buffer_size.x * uniforms.frame_buffer_size.y;
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

    let ray = ray_to_pixel(0.0, 0.0);
	let surface_intersection = trace_first_intersection(ray);
    object_id_buffer[pixel_index] = surface_intersection.object_uid;
    albedo_buffer[pixel_index] = vec4f(surface_intersection.albedo, 1.0f);
    normal_buffer[pixel_index] = vec4f(surface_intersection.normal, 0.0f);
}

@compute @workgroup_size(WORK_GROUP_SIZE_X, WORK_GROUP_SIZE_Y, 1) fn compute_color_buffer(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,) {

	let pixel_index = evaluate_pixel_index(global_invocation_id, num_workgroups);

    if (pixel_outside_frame_buffer(pixel_index)) {
        return;
    }

	setup_pixel_coordinates(pixel_index);
	setup_camera();

	randState = pixel_index + u32(uniforms.frame_number) * 719393;

	get_lights();

	let traced_color = pathTrace();

	if(uniforms.if_reset_frame_buffer == 0) {
		pixel_color_buffer[pixel_index] = vec4f(pixel_color_buffer[pixel_index].xyz + traced_color, 1.0);
	} else {
	    pixel_color_buffer[pixel_index] = vec4f(traced_color, 1.0);
	}
}

/// shootRay

// To get pixel center ->
//		x = aspect * (2 * (x / width) - 1) 	[ranges from -aspect to +aspect]
//		y = -(2 * (y / height) - 1)			[ranges from +1 to -1]

fn ray_to_pixel(subPixelX: f32, subPixelY: f32) -> Ray {
    return getCameraRay(
        (uniforms.frame_buffer_size.x / uniforms.frame_buffer_size.y) * (2 * ((pixelCoords.x - 0.5 + subPixelX) / uniforms.frame_buffer_size.x) - 1),
        -1 * (2 * ((pixelCoords.y - 0.5 + subPixelY) / uniforms.frame_buffer_size.y) - 1)
        );
}

fn pathTrace() -> vec3f {

	var pixColor = vec3f(0, 0, 0);

	if(STRATIFY)
	{
		let sqrt_spp = sqrt(NUM_SAMPLES);
		let recip_sqrt_spp = 1.0 / f32(i32(sqrt_spp));
		var numSamples = 0.0;	// NUM_SAMPLES may not be perfect square

		for(var i = 0.0; i < sqrt_spp; i += 1.0)
		{
			for(var j = 0.0; j < sqrt_spp; j += 1.0)
			{
                let ray = ray_to_pixel(recip_sqrt_spp * (i + rand2D()), recip_sqrt_spp * (j + rand2D()));
				pixColor += ray_color(ray);
				numSamples += 1;
			}
		}
		pixColor /= numSamples;
	}
	else
	{
		for(var i = 0; i < i32(NUM_SAMPLES); i += 1)
		{
		    let ray = ray_to_pixel(rand2D(), rand2D());
			pixColor += ray_color(ray);
		}
		pixColor /= NUM_SAMPLES;
	}

	return pixColor;
}

fn trace_first_intersection(ray : Ray) -> FirstHitSurface {
    var closest_so_far = MAX_FLOAT;
    var hit_uid: u32 = 0;
    var hit_albedo: vec3f = vec3f(0.0f, 0.0f, 0.0f);
    var hit_normal: vec3f = vec3f(0.0f, 0.0f, 0.0f);

    for(var i = u32(0); i < uniforms.spheres_count; i++){
        let sphere = sphere_objs[i];
        if(hit_sphere(sphere, ray_tmin, closest_so_far, ray)){
            hit_uid = sphere.object_uid;
            hit_albedo = materials[u32(sphere.material_id)].color;
            hit_normal = hitRec.normal;
            closest_so_far = hitRec.t;
        }
    }

    for(var i = u32(0); i < uniforms.parallelograms_count; i++){
        let parallelogram = quad_objs[i];
        if(hit_quad(parallelogram, ray_tmin, closest_so_far, ray)) {
            hit_uid = parallelogram.object_uid;
            hit_albedo = materials[u32(parallelogram.material_id)].color;
            hit_normal = hitRec.normal;
            closest_so_far = hitRec.t;
        }
    }

    for(var i = u32(0); i < uniforms.sdf_count; i++){
        let sdf = sdf[i];
        if(hit_sdf(sdf, ray_tmin, closest_so_far, ray)){
            hit_uid = sdf.object_uid;
            hit_albedo = materials[u32(sdf.material_id)].color;
            hit_normal = hitRec.normal;
            closest_so_far = hitRec.t;
        }
    }

    const leafNode = 2;		// fix this hardcoding later
    var invDir = 1 / ray.dir;
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
                        hit_albedo = materials[u32(triangle.material_id)].color;
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
                if(ray.dir[i32(node.axis)] < 0) {
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

fn getCameraRay(s : f32, t : f32) -> Ray {
	let eye_to_pixel_direction = (uniforms.view_matrix * vec4f(vec3f(s, t, -fovFactor), 0)).xyz;

	let pixel_world_space = vec4(cam_origin + eye_to_pixel_direction, 1.0f);
	let ray_origin_world_space = uniforms.view_ray_origin_matrix * pixel_world_space;

	let ray = Ray(ray_origin_world_space.xyz, normalize((pixel_world_space - ray_origin_world_space).xyz));

	return ray;
}

/// hitRay

fn hitScene(ray : Ray) -> bool
{
	var closest_so_far = MAX_FLOAT;
	var hit_anything = false;

	for(var i = u32(0); i < uniforms.spheres_count; i++)
	{
		let medium = materials[i32(sphere_objs[i].material_id)].material_type;
		if(medium < ISOTROPIC)
		{
			if(hit_sphere(sphere_objs[i], ray_tmin, closest_so_far, ray))
			{
				hit_anything = true;
				closest_so_far = hitRec.t;
			}
		}
		else
		{
			if(hit_volume(sphere_objs[i], ray_tmin, closest_so_far, ray))
			{
				hit_anything = true;
				closest_so_far = hitRec.t;
			}
		}
	}

	for(var i = u32(0); i < uniforms.parallelograms_count; i++)
	{
		if(hit_quad(quad_objs[i], ray_tmin, closest_so_far, ray))
		{
			hit_anything = true;
			closest_so_far = hitRec.t;
		}
	}

	// traversing BVH using a stack implementation
	// https://pbr-book.org/3ed-2018/Primitives_and_Intersection_Acceleration/Bounding_Volume_Hierarchies#CompactBVHForTraversal

	const leafNode = 2;		// fix this hardcoding later
	var invDir = 1 / ray.dir;
	var toVisitOffset = 0;
	var curNodeIdx = 0;
	var node = bvh[curNodeIdx];

	while(true) {
		node = bvh[curNodeIdx];

		if(hit_aabb(node.min, node.max, ray_tmin, closest_so_far, ray, invDir))
		{
			if(i32(node.prim_type) == leafNode)
			{
				let startPrim = i32(node.prim_id);
				let countPrim = i32(node.prim_count);
				for(var j = 0; j < countPrim; j++)
				{
					if(hit_triangle(triangles[startPrim + j], ray_tmin, closest_so_far, ray))
					{
						hit_anything = true;
						closest_so_far = hitRec.t;
					}
				}

				if(toVisitOffset == 0)
				{
					break;
				}
				toVisitOffset--;
				curNodeIdx = stack[toVisitOffset];
			}

			else
			{
				if(ray.dir[i32(node.axis)] < 0)
				{
					stack[toVisitOffset] = curNodeIdx + 1;
					toVisitOffset++;
					curNodeIdx = i32(node.right_offset);
				}
				else
				{
					stack[toVisitOffset] = i32(node.right_offset);
					toVisitOffset++;
					curNodeIdx++;
				}
			}
		}
		else
		{
			if(toVisitOffset == 0)
			{
				break;
			}

			toVisitOffset--;
			curNodeIdx = stack[toVisitOffset];
		}

		if(toVisitOffset >= STACK_SIZE)
		{
			break;
		}
	}

	for(var i = u32(0); i < uniforms.sdf_count; i++)
    {
        if(hit_sdf(sdf[i], ray_tmin, closest_so_far, ray))
        {
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

// 	var invDir = 1 / ray.dir;
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

// 	for(var i = 0; i < uniforms.spheres_count; i++)
// 	{
// 		if(hit_sphere(sphere_objs[i], ray_tmin, closest_so_far, ray))
// 		{
// 			hit_anything = true;
// 			closest_so_far = hitRec.t;
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



// fn hit_bruteForce(ray : Ray) -> bool
// {
// 	var closest_so_far = MAX_FLOAT;
// 	var hit_anything = false;

// 	for(var i = 0; i < uniforms.triangles_count; i++)
// 	{
// 		if(hit_triangle(triangles[i], ray_tmin, closest_so_far, ray))
// 		{
// 			hit_anything = true;
// 			closest_so_far = hitRec.t;
// 		}
// 	}

// 	for(var i = 0; i < uniforms.spheres_count; i++)
// 	{
// 		if(hit_sphere(sphere_objs[i], ray_tmin, closest_so_far, ray))
// 		{
// 			hit_anything = true;
// 			closest_so_far = hitRec.t;
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

fn ray_color(incidentRay : Ray) -> vec3f {

	var currRay = incidentRay;
	var acc_radiance = vec3f(0);	// initial radiance (pixel color) is black
	var throughput = vec3f(1);		// initial throughput is 1 (no attenuation)
	let background_color = vec3f(0.1, 0.1, 0.1);

	for(var i = 0; i < MAX_BOUNCES; i++)
	{
		if(hitScene(currRay) == false)
		{
			acc_radiance += (background_color * throughput);
			break;
		}

		// unidirectional light
		var emissionColor = hitRec.material.emissionColor;
		if(!hitRec.front_face) {
			emissionColor = vec3f(0);
		}

		if(IMPORTANCE_SAMPLING)
		{
			// IMPORTANCE SAMPLING TOWARDS LIGHT
			// diffuse scatter ray
			let scatterred_surface = material_scatter(currRay);

			if(scatterRec.skip_pdf) {
				acc_radiance += emissionColor * throughput;
				throughput *= mix(hitRec.material.color, hitRec.material.specularColor, doSpecular);

				currRay = scatterRec.skip_pdf_ray;
				continue;
			}

			// ray sampled towards light
			let scattered_light = get_random_on_quad(lights, hitRec.p);

			var scattered = scattered_light;
			var rand = rand2D();
			if(rand > 0.2) {
				scattered = scatterred_surface;
			}

			let lambertian_pdf = onb_lambertian_scattering_pdf(scattered);
			let light_pdf = light_pdf(scattered, lights);
			let pdf = 0.2 * light_pdf + 0.8 * lambertian_pdf;

			if(pdf <= 0.00001) {
				return emissionColor * throughput;
			}

			acc_radiance += emissionColor * throughput;
			throughput *= ((lambertian_pdf * mix(hitRec.material.color, hitRec.material.specularColor, doSpecular)) / pdf);
			currRay = scattered;
		}

		else
		{
			let scattered = material_scatter(currRay);

			acc_radiance += emissionColor * throughput;
			throughput *= mix(hitRec.material.color, hitRec.material.specularColor, doSpecular);

			currRay = scattered;
		}

		// russian roulette
		if(i > 2)
		{
			let p = max(throughput.x, max(throughput.y, throughput.z));
			if(rand2D() > p) {
				break;
			}

			throughput *= (1.0 / p);
		}
	}

	return acc_radiance;
}

/// scatterRay

var<private> doSpecular : f32;
fn material_scatter(ray_in : Ray) -> Ray {

	var scattered = Ray(vec3f(0), vec3f(0));
	doSpecular = 0;
	if(hitRec.material.material_type == LAMBERTIAN) {

		let uvw = onb_build_from_w(hitRec.normal);
		var diffuse_dir = cosine_sampling_wrt_Z();
		diffuse_dir = normalize(onb_get_local(diffuse_dir));

		scattered = Ray(hitRec.p, diffuse_dir);

		doSpecular = select(0.0, 1.0, rand2D() < hitRec.material.specularStrength);

		var specular_dir = reflect(ray_in.dir, hitRec.normal);
		specular_dir = normalize(mix(specular_dir, diffuse_dir, hitRec.material.roughness));

		scattered = Ray(hitRec.p, normalize(mix(diffuse_dir, specular_dir, doSpecular)));

		scatterRec.skip_pdf = false;

		if(doSpecular == 1.0) {
			scatterRec.skip_pdf = true;
			scatterRec.skip_pdf_ray = scattered;
		}
	}

	else if(hitRec.material.material_type == MIRROR) {
		var reflected = reflect(ray_in.dir, hitRec.normal);
		scattered = Ray(hitRec.p, normalize(reflected + hitRec.material.roughness * uniform_random_in_unit_sphere()));

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}

	else if(hitRec.material.material_type == GLASS) {
		var ir = hitRec.material.eta;
		if(hitRec.front_face == true) {
			ir = (1.0 / ir);
		}

		let unit_direction = ray_in.dir;
		let cos_theta = min(dot(-unit_direction, hitRec.normal), 1.0);
		let sin_theta = sqrt(1 - cos_theta*cos_theta);

		var direction = vec3f(0);
		if(ir * sin_theta > 1.0 || reflectance(cos_theta, ir) > rand2D()) {
			direction = reflect(unit_direction, hitRec.normal);
		}
		else {
			direction = refract(unit_direction, hitRec.normal, ir);
		}

		if(near_zero(direction)) {
			direction = hitRec.normal;
		}

		scattered = Ray(hitRec.p, direction);

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}

	else if(hitRec.material.material_type == ISOTROPIC) {
		let g = hitRec.material.specularStrength;
		let cos_hg = (1 + g*g - pow(((1 - g*g) / (1 - g + 2*g*rand2D())), 2.0)) / (2 * g);
		let sin_hg = sqrt(1 - cos_hg * cos_hg);
		let phi = 2 * PI * rand2D();

		let hg_dir = vec3f(sin_hg * cos(phi), sin_hg * sin(phi), cos_hg);

		let uvw = onb_build_from_w(ray_in.dir);
		scattered = Ray(hitRec.p, normalize(onb_get_local(hg_dir)));

		scatterRec.skip_pdf = true;
		scatterRec.skip_pdf_ray = scattered;
	}

	return scattered;
}

/// importanceSampling

fn reflectance(cosine : f32, ref_idx : f32) -> f32 {
	var r0 = (1 - ref_idx) / (1 + ref_idx);
	r0 = r0 * r0;
	return r0 + (1 - r0) * pow((1 - cosine), 5.0);
}

fn uniform_random_in_unit_sphere() -> vec3f {
	let phi = rand2D() * 2.0 * PI;
	let theta = acos(2.0 * rand2D() - 1.0);

	let x = sin(theta) * cos(phi);
	let y = sin(theta) * sin(phi);
	let z = cos(theta);

	return vec3f(x, y, z);
}

fn random_in_unit_disk() -> vec3f {
	let theta = 2 * PI * rand2D();
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
	let r1 = rand2D();
	let r2 = rand2D();

	let phi = 2 * PI * r1;
	let x = cos(phi) * sqrt(r2);
	let y = sin(phi) * sqrt(r2);
	let z = sqrt(1 - r2);

	return vec3f(x, y, z);
}

fn lambertian_scattering_pdf(scattered : Ray) -> f32 {
	let cos_theta = max(0.0, dot(hitRec.normal, scattered.dir));
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
	let cosine_theta = dot(normalize(scattered.dir), unit_w);
	return max(0.0, cosine_theta/PI);
}

fn get_random_on_quad(q : Parallelogram, origin : vec3f) -> Ray {
	let p = q.Q + (rand2D() * q.u) + (rand2D() * q.v);
	return Ray(origin, normalize(p - origin));
}

fn get_random_on_quad_point(q : Parallelogram) -> vec3f {
	let p = q.Q + (rand2D() * q.u) + (rand2D() * q.v);
	return p;
}

fn light_pdf(ray : Ray, quad : Parallelogram) -> f32 {

	if(dot(ray.dir, quad.normal) > 0) {
		return MIN_FLOAT;
	}

	let denom = dot(quad.normal, ray.dir);

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
	let front_face = dot(ray.dir, quad.normal) < 0;
	if(front_face == false)
	{
		hitNormal = -hitNormal;
	}

	let distance_squared = t * t * length(ray.dir) * length(ray.dir);
	let cosine = abs(dot(ray.dir, hitNormal) / length(ray.dir));

	return (distance_squared / (cosine * length(cross(lights.u, lights.v))));
}

/// vertex

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

/// fragment

fn pixel_global_index(pixel_position: vec2f) -> u32 {
    return (u32(pixel_position.y) * u32(uniforms.frame_buffer_size.x) + u32(pixel_position.x));
}

@fragment fn fs(@builtin(position) fragment_coordinate: vec4f) -> @location(0) vec4f {
    let i = pixel_global_index(fragment_coordinate.xy);
    var color = pixel_color_buffer[i].xyz / uniforms.frame_number;

    color = aces_approx(color.xyz);
    color = pow(color.xyz, vec3f(1/2.2));

    if(uniforms.if_reset_frame_buffer == 1) {
        pixel_color_buffer[i] = vec4f(0);
    }

    return vec4f(color, 1);
}
