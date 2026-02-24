const EPS: f32 = 0.000001;
const SKY: vec3f = vec3f(0.53, 0.81, 0.92);

//Base info
struct Uniform {
    world_from_clip: mat4x4f,
    resolution: vec2f,
    _pad: vec2i, // fills the gap
    render_distance: f32,
    voxel_size: f32,
    inv_voxel_size: f32,
    buffer_size: u32,
    max_height: f32,
}


//Information about a single quad
struct GpuQuadInfo {
    world_coords: vec2f, //coordinates of the vertex with the lowest (closest to negative infinity) x and z values in world space
    voxel_coords: vec2i,
    upper: GpuSimplePlane,
    lower: GpuSimplePlane,
    y_max: f32,
    y_min: f32,
    _pad: vec2i, // 8 bytes to reach 96
}

//information about a single plane
struct GpuSimplePlane {
    n: vec3f,
    d: f32,
    material_id: i32,
    _pad: vec3i, // 12 bytes to reach 32
}

//Material properties
struct GpuMaterial {
    base_color: vec3f,
    reflectance: f32,
    emissive: vec3f,
    roughness: f32,
    specular_tint: vec3f,
    metallic: f32,
    attenuation_color: vec3f,
    diffuse_transmission: f32,
    specular_transmission: f32,
    ior: f32, 
    thickness: f32,
    attenuation_distance: f32,
    anisotropy_strength: f32,
    anisotropy_rotation: f32,
    clearcoat: f32,
    clearcoat_roughness: f32,
    emissive_strength: f32,
    _pad: i32, // 4 bytes
    _pad2: vec2i, // 8 bytes — reaches 112
}

struct HitInfo {
    pos: vec3f,
    material_id: i32,
    normal: vec3f,
}

@group(0) @binding(0) 
var<uniform> unif: Uniform;

@group(0) @binding(1)
var<storage> quads: array<GpuQuadInfo>;

@group(1) @binding(0)
var<uniform> materials: array<GpuMaterial, 1>;

@fragment
fn frag_main(@builtin(position) frag_coords: vec4f) -> @location(0) vec4f {
    let uv = frag_coords.xy / unif.resolution;

    //normalized device coordinates (flip y)
    let ndc = vec2f(
        uv.x * 2.0 - 1.0,
        1.0 - uv.y * 2.0
    );

    let clip_near = vec4f(ndc, -1.0, 1.0);
    let clip_far  = vec4f(ndc,  1.0, 1.0);

    let world_near = unif.world_from_clip * clip_near;
    let world_far  = unif.world_from_clip * clip_far;

    let origin = world_near.xyz / world_near.w;
    let far = world_far.xyz / world_far.w;

    let direction = normalize(far - origin);

    let hit = traverse(origin, direction);

    if hit.material_id == -1 {
        return vec4f(SKY,1);
    }

    let color = materials[hit.material_id].base_color;
    return vec4f(color,1);
}

fn traverse(origin: vec3f, dir: vec3f) -> HitInfo {
    let ray_dir_xz = dir.xz;
    let t_max = unif.render_distance / length(ray_dir_xz);
    var current_voxel = to_voxel(origin);
    let ray_origin_y = origin.y;
    var ray_end_y = origin.y;
    let ray_dir_y = dir.y;

    let tilted_up = ray_dir_y > 0;

    if ray_origin_y > unif.max_height {
        // Ray pointing up or horizontal while above max height, will never hit terrain
        if ray_dir_y >= 0.0 {
            return no_hit();
        }

        // Ray pointing down - if it won't dip below max height until outside of render distance, will never hit terrain
        let t_to_max_height = (unif.max_height - ray_origin_y) / ray_dir_y;
        if t_to_max_height > t_max {
            return no_hit();
        }
    }

    //Near vertical ray
    if length(ray_dir_xz) < EPS {
        if ray_dir_y > 0 {
            return no_hit();
        }

        let idx = get_index(current_voxel.x,current_voxel.y);
        let quad = quads[idx];
        let low_hit = check_lower(origin, dir, quad);
        if low_hit.material_id != -1 {
            return low_hit;
        } else {
            return check_upper(origin, dir, quad);
        }
    }

    let offset: vec2f = vec2f(
        select(0.0, 1.0, dir.x > 0.0),
        select(0.0, 1.0, dir.z > 0.0),
    );

    let next_boundary: vec2f = vec2f(
        (f32(current_voxel.x) + offset.x) * unif.voxel_size,
        (f32(current_voxel.y) + offset.y) * unif.voxel_size,
    );

    //must be initalized per component so we can insert INFINITY to avoid NaN
    var t_vec = vec2f(0,0);
    var delta = vec2f(0,0);

    // X axis
    if ray_dir_xz.x != 0.0 {
        let inv = 1.0 / ray_dir_xz.x;
        delta.x = unif.voxel_size * abs(inv);
        t_vec.x = (next_boundary.x - origin.x) * inv;
    } else {
        delta.x = 1e30;
        t_vec.x = 1e30;
    }

    // Z axis
    if ray_dir_xz.y != 0.0 {
        let inv = 1.0 / ray_dir_xz.y;
        delta.y = unif.voxel_size * abs(inv);
        t_vec.y = (next_boundary.y - origin.z) * inv;
    } else {
        delta.y = 1e30;
        t_vec.y = 1e30;
    }

    let step: vec2i = vec2i(
    i32(sign(dir.x)),
    i32(sign(dir.z)),
    );

    var t_current = 0.0;
    loop {
        //If our ray is tilted up and is above the highest known terrain, we can stop marching as it will never collide
        if tilted_up && ray_end_y > unif.max_height {
            break;
        }

        let t_next = min(t_vec.x,t_vec.y);

        let enter_point = origin + dir * t_current;
        let exit_point = origin + dir * t_next;

        let idx = get_index(current_voxel.x, current_voxel.y);
        let quad = quads[idx];
        let some_hit = intersect(origin, dir, quad, enter_point, exit_point);

        if some_hit.material_id != -1 {
            return some_hit;
        }

        //See which quad edge we intersected first
        if t_vec.x < t_vec.y {
            //intersected with x plane first
            current_voxel.x += step.x;
            t_vec.x += delta.x;
        } else {
            //intersected with z plane first
            current_voxel.y += step.y;
            t_vec.y += delta.y;
        }

        t_current = t_next;
        ray_end_y = ray_origin_y + t_current * ray_dir_y;

        if t_current > t_max {
            //reached render distance
            break;
        }
    }
    return no_hit();
}

///converts world position to voxel coordinates
fn to_voxel(pos: vec3f) -> vec2i {
    return vec2i(floor(pos.xz * unif.inv_voxel_size));
}

///returns a blank hit struct with a material value of -1, signifying no hit
fn no_hit() -> HitInfo {
    return HitInfo(
        vec3f(0,0,0),
        -1,
        vec3f(0,0,0),
    );
}

fn positive_mod(a: i32, b: i32) -> u32 {
    return u32(((a % b) + b) % b);
}

//Takes the x and z voxel coordinates of a quad and returns the index within the terrain buffer
fn get_index(x: i32, z: i32) -> u32 {
    return positive_mod(z, i32(unif.buffer_size)) * unif.buffer_size + positive_mod(x, i32(unif.buffer_size));
}

fn ray_plane(origin: vec3f, dir: vec3f, plane: GpuSimplePlane) -> HitInfo {

    let denom = dot(plane.n,dir);

    // Parallel (or extremely close to parallel)
    if abs(denom) < EPS {
        return no_hit();
    }

    let t = -(dot(plane.n,origin) + plane.d) / denom;

    if t < EPS {
        return no_hit();
    }

    return HitInfo((origin + t * dir),plane.material_id,plane.n);
}

fn check_upper(origin: vec3f, dir: vec3f, quad: GpuQuadInfo) -> HitInfo {
    let hit = ray_plane(origin, dir, quad.upper);

    if hit.material_id != -1 {
        if any(to_voxel(hit.pos) != quad.voxel_coords) {
            return no_hit();
        }

        let hit_local = hit.pos.xz - quad.world_coords;
        if hit_local.x + hit_local.y >= unif.voxel_size {
            return hit;
        }
    }
    return no_hit();
}

fn check_lower(origin: vec3f, dir: vec3f, quad: GpuQuadInfo) -> HitInfo {
    let hit = ray_plane(origin, dir, quad.lower);

    if hit.material_id != -1 {
        if any(to_voxel(hit.pos) != quad.voxel_coords) {
            return no_hit();
        }

        let hit_local = hit.pos.xz - quad.world_coords;
        if hit_local.x + hit_local.y <= unif.voxel_size {
            return hit;
        }
    }
    return no_hit();
}

fn intersect(origin: vec3f, dir: vec3f, quad: GpuQuadInfo, enter: vec3f, exit: vec3f) -> HitInfo {
    if (enter.y > quad.y_max && exit.y > quad.y_max)
        || (enter.y < quad.y_min && exit.y < quad.y_min)
    {
        return no_hit(); //fully above terrain or below terrain
    }

    //check lower first
    if enter.z < exit.z || enter.x < exit.x {
        let low_hit = check_lower(origin, dir, quad);
        if low_hit.material_id != -1 {
            return low_hit;
        } else {
            return check_upper(origin, dir, quad);
        }
    //check upper first
    } else {
        let up_hit = check_upper(origin, dir, quad);
        if up_hit.material_id != -1 {
            return up_hit;
        } else {
            return check_lower(origin, dir, quad);
        }
    }
}