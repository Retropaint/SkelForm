struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) add_color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) add_color: vec4<f32>,
};

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = vert.tex_coords;
    out.position = vert.position;
    out.color = vert.color;
    out.add_color = vert.add_color;
    return out;
};

// fragment

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return (tex * in.color) + in.add_color;
}
