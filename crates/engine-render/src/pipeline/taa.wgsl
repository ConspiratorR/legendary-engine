// ── Temporal Anti-Aliasing (TAA) Resolve ───────────────────────────
// Blends current frame with history buffer using motion vectors.
// Uses neighborhood clamping to reduce ghosting artifacts.

struct TaaParams {
    blend_factor: f32,     // How much of current frame to use (0.05 = 95% history)
    jitter_scale: f32,     // Sub-pixel jitter scale
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var current_texture: texture_2d<f32>;

@group(0) @binding(1)
var history_texture: texture_2d<f32>;

@group(0) @binding(2)
var motion_texture: texture_2d<f32>;

@group(0) @binding(3)
var tex_sampler: sampler;

@group(0) @binding(4)
var<uniform> params: TaaParams;

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

// Neighborhood clamping: clamp history color to min/max of 3x3 neighborhood
fn neighborhood_clamp(current: vec2<f32>, color: texture_2d<f32>, samp: sampler) -> vec3<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(color));

    var min_color = vec3<f32>(1e10);
    var max_color = vec3<f32>(-1e10);

    // 3x3 neighborhood
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2(f32(x), f32(y)) * texel;
            let sample_color = textureSample(color, samp, current + offset).rgb;
            min_color = min(min_color, sample_color);
            max_color = max(max_color, sample_color);
        }
    }

    return clamp(textureSample(color, samp, current).rgb, min_color, max_color);
}

@fragment
fn fs_taa(input: VertexOutput) -> @location(0) vec4<f32> {
    let current = textureSample(current_texture, tex_sampler, input.uv);
    let motion = textureSample(motion_texture, tex_sampler, input.uv).xy;

    // Reproject to history buffer using motion vectors
    let history_uv = input.uv - motion;
    let history = neighborhood_clamp(history_uv, current_texture, tex_sampler);

    // Blend: mostly history, small amount of current for responsiveness
    let blended = mix(history, current.rgb, params.blend_factor);

    return vec4(blended, current.a);
}
