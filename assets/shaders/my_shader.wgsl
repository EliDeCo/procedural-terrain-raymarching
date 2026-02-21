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
    material_id: u32,
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


//Base info
struct Uniform {
    world_from_clip: mat4x4f,
    resolution: vec2f,
    _pad: vec2i, // fills the gap
}

//holds all currently generated quads and material info
struct GpuTerrainStore {
    materials: array<GpuMaterial, 1>,
    quad_buffer: array<GpuQuadInfo>
}


@group(0) @binding(0) 
var<uniform> unif: Uniform;

@fragment
fn frag_main(@builtin(position) frag_coords: vec4f) -> @location(0) vec4f {
    let uv = frag_coords.xy / unif.resolution;

    //flip y
    let ndc = vec2f(
        uv.x * 2.0 - 1.0,
        1.0 - uv.y * 2.0
    );

    let clip_near = vec4f(ndc, -1.0, 1.0);
    let clip_far  = vec4f(ndc,  1.0, 1.0);

    let world_near = unif.world_from_clip * clip_near;
    let world_far  = unif.world_from_clip * clip_far;

    let p0 = world_near.xyz / world_near.w; //doubles as ray origin
    let p1 = world_far.xyz / world_far.w;

    let dir = normalize(p1 - p0);

    return vec4f(dir,1);
    //return vec4(unif.materials[0].base_color,1);
}

//Eric Green: (0.36,0.64,0.29,1);