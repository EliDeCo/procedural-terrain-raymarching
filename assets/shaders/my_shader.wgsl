const FULLSCREEN = array<vec2<f32>, 3>(
  vec2(-1.0, -1.0),
  vec2( 3.0, -1.0),
  vec2(-1.0,  3.0),
);

/* 
@vertex
fn vtx_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    return vec4f(FULLSCREEN[index],0,1);
}
*/

@fragment
fn frag_main() -> @location(0) vec4f {
    return vec4(0.36,0.64,0.29,1);
}