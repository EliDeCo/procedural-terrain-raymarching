const FULLSCREEN = array<vec2<f32>, 3>(
  vec2(-1.0, -1.0),
  vec2( 3.0, -1.0),
  vec2(-1.0,  3.0),
);

struct Uniform {
    resolution: vec2f
}
@group(0) @binding(0) 
var<uniform> unif: Uniform;

@fragment
fn frag_main(@builtin(position) frag_coords: vec4f) -> @location(0) vec4f {
    let pixel_coords = frag_coords.xy / unif.resolution;

    let av = (pixel_coords.x + pixel_coords.y) *0.5;

    return vec4(pixel_coords.x,pixel_coords.y,0,av);
    //return vec4(0.36,0.64,0.29,1); Eric Green
}

//Eric Green: (0.36,0.64,0.29,1);