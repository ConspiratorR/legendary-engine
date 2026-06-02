// ── Composite ─────────────────────────────────────────────────────
// General-purpose compositing pass for applying post-processing results
// back to the active HDR buffer.
//
// Modes:
//   0 = multiply  — dst.rgb * src.r   (SSAO application)
//   1 = copy      — src.rgb           (TAA/fog resolve)
//   2 = additive  — dst.rgb + src.rgb * intensity  (SSR/volumetric)

struct CompositeParams {
    mode: u32,
    intensity: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var src_texture: texture_2d<f32>;

@group(0) @binding(1)
var dst_texture: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

@group(0) @binding(3)
var<uniform> params: CompositeParams;

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
fn fs_composite(input: VertexOutput) -> @location(0) vec4<f32> {
    let src = textureSample(src_texture, tex_sampler, input.uv);
    let dst = textureSample(dst_texture, tex_sampler, input.uv);

    var result: vec3<f32>;
    if (params.mode == 0u) {
        // Multiply: apply SSAO occlusion (src.r = 1 where lit, 0 where occluded)
        result = dst.rgb * src.r;
    } else if (params.mode == 1u) {
        // Copy: replace destination with source
        result = src.rgb;
    } else {
        // Additive: blend source onto destination
        result = dst.rgb + src.rgb * params.intensity;
    }
    return vec4(result, dst.a);
}
