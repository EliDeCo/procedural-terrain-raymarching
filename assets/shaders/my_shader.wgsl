const EPS: f32 = 0.01;
const SKY: vec3f = vec3f(0.53, 0.81, 0.92);
const LIGHT_DIR: vec3f = (vec3f(0.6533, -0.3827, -0.6533));
//const LIGHT_DIR: vec3f = (vec3f(0.5, -0.70710678, -0.5));
const LIGHT_DIR_INV: vec3f = -LIGHT_DIR;
const LIGHT_ANGULAR_SIZE: f32 = 0.05; // angular size of the sun/moon in radians
const AMBIENT = 0.05;

//Base info
struct Uniform {
    world_from_clip: mat4x4f,
    resolution: vec2f,
    buffer_mask: i32,
    buffer_shift: u32,
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
    _pad: vec2i,
    n1: vec4f, //vertex normal of the vertex at world_coords (x_min, z_min). 4th value is unused
    pos_1: vec4f, //The 3d position of the vertex at world_coords (x_min, z_min). 4th value is unused
    n2: vec4f, //vertex normal of the vertex at world_coords (x_max, z_min). 4th value is unused
    pos_2: vec4f, //The 3d position of the vertex at world_coords (x_max, z_min). 4th value is unused
}

//information about a single plane
struct GpuSimplePlane {
    n_and_d: vec4f, //first 3 is normal, last entry is plane constant d
    material_id: vec4i, //only first entry is valid
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
    pos: vec3f, //hit position
    material_id: i32, //id of the hit material
    normal: vec3f, //surface normal at hit position
    voxel: vec2i, //voxel coordinates of the hit quad
}

@group(0) @binding(0) 
var<uniform> unif: Uniform;

@group(0) @binding(1)
var<storage> quads: array<GpuQuadInfo>;

@group(0) @binding(2)
var mipmap: texture_2d<f32>;

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


    let diffuse = max(0,dot(hit.normal,LIGHT_DIR_INV));
    let shadow =  traverse_shadow(hit.pos + 0.01 * LIGHT_DIR_INV,LIGHT_DIR_INV);
    let final_color = materials[hit.material_id].base_color * (AMBIENT + diffuse * shadow);

    return vec4f(final_color,1);
    //let value = sample_mipmap(hit.voxel, 0);

    //return vec4f(vec3f((value+121)/242),1);
}

//voxel is the voxel coordinates of the location we want to sample
fn sample_mipmap(voxel_coord: vec2i, level: u32) -> f32 {
    let texel = (voxel_coord & vec2i(unif.buffer_mask)) >> vec2u(level);
    return textureLoad(mipmap, texel, level).x;
}

fn traverse(origin: vec3f, dir: vec3f) -> HitInfo {
    let ray_dir_xz = dir.xz;
    let t_max = unif.render_distance / length(ray_dir_xz);
    var current_voxel = to_voxel(origin);
    let ray_origin_y = origin.y;
    var ray_end_y = origin.y;
    let ray_dir_y = dir.y;
    var t_current = 0.0;
    let tilted_up = ray_dir_y > 0;
    
    if ray_origin_y > unif.max_height {
        // Ray pointing up or horizontal while above max height, will never hit terrain
        if ray_dir_y >= 0.0 {
            return no_hit();
        }

        // Advance ray to where it first hits max_height
        let t_to_max_height = (unif.max_height - ray_origin_y) / ray_dir_y;
        if t_to_max_height > t_max {
            return no_hit();
        }

        // Skip ahead to that point
        t_current = t_to_max_height;
        ray_end_y = unif.max_height;
        current_voxel = to_voxel(origin + dir * t_current);
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

    //must be initalized per component to avoid dividing by 0
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

    loop {
        //If our ray is tilted up and is above the highest known terrain, we can stop marching as it will never collide
        if tilted_up && ray_end_y > unif.max_height {
            return no_hit();
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
            return no_hit();
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
        vec2i(0,0)
    );
}

fn positive_mod(a: i32, b: i32) -> u32 {
    return u32(((a % b) + b) % b);
}

//Takes the x and z voxel coordinates of a quad and returns the index within the terrain buffer
fn get_index(x: i32, z: i32) -> u32 {
    let xi = u32(x & unif.buffer_mask);
    let zi = u32(z & unif.buffer_mask);
    return (zi << unif.buffer_shift) | xi;
}
//                                                      upper => true, lower => false
fn ray_plane(origin: vec3f, dir: vec3f, plane: GpuSimplePlane, is_upper: bool, current_quad: GpuQuadInfo) -> HitInfo {
    let plane_n = plane.n_and_d.xyz;
    let d = plane.n_and_d.w;

    let denom = dot(plane_n,dir);

    // Parallel (or extremely close to parallel)
    if abs(denom) < EPS {
        return no_hit();
    }

    let t = -(dot(plane_n,origin) + d) / denom;

    if t < EPS {
        return no_hit();
    }


    let adj_index = get_index(current_quad.voxel_coords.x,current_quad.voxel_coords.y+1);
    let adj_quad = quads[adj_index];

    let n0 = adj_quad.n1.xyz;
    let p0 = adj_quad.pos_1.xyz;
    let n1 = select(current_quad.n2, adj_quad.n2, is_upper).xyz;
    let p1 = select(current_quad.pos_2, adj_quad.pos_2, is_upper).xyz;
    let n2 = select(current_quad.n1, current_quad.n2, is_upper).xyz;
    let p2 = select(current_quad.pos_1, current_quad.pos_2, is_upper).xyz;

    let point = origin + t * dir;
    let bary = barycentric(point, p0, p1, p2);
    let smooth_normal = normalize(n0 * bary.x + n1 * bary.y + n2 * bary.z);
    //let smooth_normal = vec3f(bary.x, bary.y, bary.z);

    return HitInfo(
        point,
        plane.material_id.x,
        smooth_normal,
        current_quad.voxel_coords
    );
}

fn barycentric(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = dot(v0, v0);
    let d01 = dot(v0, v1);
    let d11 = dot(v1, v1);
    let d20 = dot(v2, v0);
    let d21 = dot(v2, v1);
    let inv_denom = 1 /(d00 * d11 - d01 * d01);
    let v = (d11 * d20 - d01 * d21) * inv_denom;
    let w = (d00 * d21 - d01 * d20) * inv_denom;
    let u = 1.0 - v - w;
    return vec3<f32>(u, v, w);
}

fn check_upper(origin: vec3f, dir: vec3f, quad: GpuQuadInfo) -> HitInfo {
    let hit = ray_plane(origin, dir, quad.upper, true, quad);

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
    let hit = ray_plane(origin, dir, quad.lower, false, quad);

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
        return no_hit();
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

///Returns a float that should be multiplied by the albedo of the terrain to get its final color
fn traverse_shadow(origin: vec3f, dir: vec3f) -> f32 {
    var t_current = 0.0;
    let ray_dir_xz = dir.xz;
    let t_max = unif.render_distance / length(ray_dir_xz);
    var current_voxel = to_voxel(origin);
    let ray_origin_y = origin.y;
    var ray_end = origin;
    let ray_dir_y = dir.y;
    let tilted_up = ray_dir_y > 0;
    var occlusion = 1.;
    var ph = 1e20;
    
    if ray_origin_y > unif.max_height {
        // Ray pointing up or horizontal while above max height, will never hit terrain
        if ray_dir_y >= 0.0 {
            return 1;
        }

        // Advance ray to where it first hits max_height
        let t_to_max_height = (unif.max_height - ray_origin_y) / ray_dir_y;
        if t_to_max_height > t_max {
            return 1;
        }

        // Skip ahead to that point
        t_current = t_to_max_height;
        ray_end = origin + dir * t_current;
        current_voxel = to_voxel(origin + dir * t_current);
    }

    //Near vertical ray (basically impossible to occlude in a heightmap)
    if length(ray_dir_xz) < EPS {
        return 1;
    }

    let offset: vec2f = vec2f(
        select(0.0, 1.0, dir.x > 0.0),
        select(0.0, 1.0, dir.z > 0.0),
    );

    let next_boundary: vec2f = vec2f(
        (f32(current_voxel.x) + offset.x) * unif.voxel_size,
        (f32(current_voxel.y) + offset.y) * unif.voxel_size,
    );

    //must be initalized per component to avoid dividing by 0
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

    loop {
        //If our ray is tilted up and is above the highest known terrain, we can stop marching as it will never collide
        if tilted_up && ray_end.y > unif.max_height {
            return occlusion;
        }

        let t_next = min(t_vec.x,t_vec.y);

        let enter_point = origin + dir * t_current;
        let exit_point = origin + dir * t_next;

        let idx = get_index(current_voxel.x, current_voxel.y);
        let quad = quads[idx];
        let some_hit = intersect(origin, dir, quad, enter_point, exit_point);

        if some_hit.material_id != -1 {
            return 0;
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
        ray_end = origin + t_current * dir;

        if t_current > t_max {
            //reached render distance
            return occlusion;
        }
        occlusion = min(occlusion, get_sdf(ray_end,current_voxel) / (t_current * LIGHT_ANGULAR_SIZE) );
    }
    return occlusion;
}


fn get_sdf(point: vec3f, voxel_coords: vec2i) -> f32 {
    let vertices = get_vertices(voxel_coords);

    let frac = (point.xz - vertices[1].xz) * unif.inv_voxel_size;

    let h_near = mix( vertices[1].y,  vertices[2].y, frac.x);
    let h_far  = mix( vertices[0].y,  vertices[3].y, frac.x);
    let height = max(0, point.y - mix(h_near, h_far, frac.y));

    return height; //not an actual sdf but much cheaper and good enough in most cases
    /* 
    //check at least "terrain_height" meters in each direction for the sdf
    //let check_radius = ceil(height * unif.inv_voxel_size);
    let check_radius =  1;
    var min_dist = height * height; //working in squared distance for now
    
    //check neighboring triangles and find the shortest distance
    for (var dz = -i32(check_radius); dz <= i32(check_radius); dz++) {
        for (var dx = -i32(check_radius); dx <= i32(check_radius); dx++) {

            let coords = voxel_coords + vec2i(dx, dz);
            let vertexes = get_vertices(coords);

            //lower triangle
            min_dist = min(min_dist, udTriangle(point,vertexes[0],vertexes[1],vertexes[2]));

            //upper triangle
            min_dist = min(min_dist, udTriangle(point,vertexes[0],vertexes[2],vertexes[3]));

        }
    }

    return max(0.0, sqrt(min_dist));
    */
}

//gets all 4 vertices of the given quad
//Wound CCW starting from top left
fn get_vertices(coords: vec2i) -> array<vec3f, 4> {
    let idx = get_index(coords.x, coords.y);
    let quad = quads[idx];
    let adj_idx = get_index(coords.x, coords.y + 1);
    let adj_quad = quads[adj_idx];

    return array<vec3f, 4>(adj_quad.pos_1.xyz,quad.pos_1.xyz,quad.pos_2.xyz,adj_quad.pos_2.xyz);
}

fn dot2(v: vec3f) -> f32 {
    return dot(v, v);
}

/* 
//Returns distance squared to the triangle
//Points must be wound CCW
fn udTriangle(p: vec3f, a: vec3f, b: vec3f, c: vec3f) -> f32 {
    let ba = b - a; let pa = p - a;
    let cb = c - b; let pb = p - b;
    let ac = a - c; let pc = p - c;
    let n = cross(ba, ac);

    return select(
        dot(n,pa)*dot(n,pa)/dot2(n),
        min(min(
            dot2(ba * clamp(dot(ba,pa)/dot2(ba),0.,1.) - pa),
            dot2(cb * clamp(dot(cb,pb)/dot2(cb),0.,1.) - pb)),
            dot2(ac * clamp(dot(ac,pc)/dot2(ac),0.,1.) - pc)),
        // sign test to determine if inside all three edges
        sign(dot(cross(ba,n),pa)) +
        sign(dot(cross(cb,n),pb)) +
        sign(dot(cross(ac,n),pc)) < 2.0
    );
}
    */