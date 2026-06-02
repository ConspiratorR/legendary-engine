// ── SSAO Depth-Aware Blur ──────────────────────────────────────────
// Bilateral blur that preserves edges using depth comparison.
// Two-pass separable blur (horizontal + vertical) for performance.

struct BlurParams {
    direction: vec2<f32>,  // (1,0) for horizontal, (0,1) for vertical
    depth_threshold: f32,
    _pad: f32,
};

@group(0) @binding(0)
var ssao_texture: texture_2d<f32>;

@group(0) @binding(1)
var position_texture: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

@group(0) @binding(3)
var<uniform> params: BlurParams;

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
fn fs_blur(input: VertexOutput) -> @location(0) f32 {
    let center_depth = textureSample(position_texture, tex_sampler, input.uv).z;
    let center_ao = textureSample(ssao_texture, tex_sampler, input.uv).r;

    // 9-tap bilateral blur with depth-aware weights
    let texel_size = 1.0 / vec2<f32>(textureDimensions(ssao_texture));
    var result = 0.0;
    var total_weight = 0.0;

    // Gaussian weights: 0.016, 0.055, 0.119, 0.186, 0.242, 0.186, 0.119, 0.055, 0.016
    let weights = array<f32, 5>(0.242, 0.186, 0.119, 0.055, 0.016);

    for (var i = -4; i <= 4; i++) {
        let offset = params.direction * texel_size * f32(i);
        let sample_uv = input.uv + offset;

        let sample_depth = textureSample(position_texture, tex_sampler, sample_uv).z;
        let sample_ao = textureSample(ssao_texture, tex_sampler, sample_uv).r;

        // Depth-aware weight: reject samples across depth discontinuities
        let depth_diff = abs(center_depth - sample_depth);
        let depth_weight = smoothstep(0.0, params.depth_threshold, depth_diff);
        let gaussian_weight = weights[abs(i)];

        let weight = gaussian_weight * (1.0 - depth_weight);
        result += sample_ao * weight;
        total_weight += weight;
    }

    return result / max(total_weight, 0.001);
}
