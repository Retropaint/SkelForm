@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let pos = array<vec2<f32>, 3>(
        vec2(-1.0, -3.0),
        vec2( 3.0,  1.0),
        vec2(-1.0,  1.0),
    );
    return vec4(pos[i], 0.0, 1.0);
}

struct BlitUniforms {
    window_x: f32,
    window_y: f32,
    magnification: f32,
};

@group(0) @binding(0)
var scene_tex: texture_2d<f32>;
@group(0) @binding(1)
var scene_sampler: sampler;
@group(0) @binding(2)
var<uniform> uniforms: BlitUniforms;

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(scene_tex));
    let uv = pos.xy / dims / uniforms.magnification;
    return textureSample(scene_tex, scene_sampler, uv);
}
