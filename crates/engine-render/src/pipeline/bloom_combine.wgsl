// ── Bloom Combine ──────────────────────────────────────────────────
// Additively blends the bloom texture onto the HDR framebuffer.

struct CombineParams {
    intensity: f32,
    _pad: vec3<f32>,
};

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;

@group(0) @binding(1)
var bloom_texture: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

@group(0) @binding(3)
var<uniform> params: CombineParams;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    output.clip_position = vec4(x, y, 0.0, 1.0);
    output.uv = vec2((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return output;
}

@fragment
fn fs_combine(input: VertexOutput) -> @location(0) vec4<f32> {
    let hdr = textureSample(hdr_texture, tex_sampler, input.uv);
    let bloom = textureSample(bloom_texture, tex_sampler, input.uv);
    return vec4(hdr.rgb + bloom.rgb * params.intensity, hdr.a);
}
