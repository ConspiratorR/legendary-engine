// ── Bloom Gaussian Blur ─────────────────────────────────────────────
// Separable Gaussian blur for bloom effect.
// Single-pass: direction parameter controls horizontal/vertical.

struct BlurParams {
    direction: vec2<f32>,  // (1,0) for horizontal, (0,1) for vertical
    radius: f32,
    _pad: f32,
};

@group(0) @binding(0)
var input_texture: texture_2d<f32>;

@group(0) @binding(1)
var tex_sampler: sampler;

@group(0) @binding(2)
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
fn fs_blur(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel_size = 1.0 / vec2<f32>(textureDimensions(input_texture));
    var result = vec3<f32>(0.0);
    var total_weight = 0.0;

    // 13-tap Gaussian kernel (sigma ~= 4.0)
    let offsets = array<f32, 7>(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
    let weights = array<f32, 7>(0.1965, 0.1749, 0.1210, 0.0656, 0.0278, 0.0092, 0.0024);

    // Center sample
    result += textureSample(input_texture, tex_sampler, input.uv).rgb * weights[0];
    total_weight += weights[0];

    // Symmetric samples
    for (var i = 1u; i < 7u; i++) {
        let offset = params.direction * texel_size * offsets[i] * params.radius;
        let sample_pos = input.uv + offset;
        let sample_neg = input.uv - offset;

        result += textureSample(input_texture, tex_sampler, sample_pos).rgb * weights[i];
        result += textureSample(input_texture, tex_sampler, sample_neg).rgb * weights[i];
        total_weight += weights[i] * 2.0;
    }

    return vec4(result / total_weight, 1.0);
}
