// ── Tone Mapping Pass ──────────────────────────────────────────────
// Full-screen triangle vertex shader + tone mapping fragment shader
// Converts HDR (Rgba16Float) framebuffer to LDR (swapchain) output.

struct TonemappingParams {
    exposure: f32,
    operator: u32,   // 0 = Reinhard, 1 = ACES, 2 = Exponential, 3 = Linear
    gamma: f32,
    _pad: f32,
};

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;

@group(0) @binding(1)
var hdr_sampler: sampler;

@group(0) @binding(2)
var<uniform> params: TonemappingParams;

// ── Full-screen triangle vertex shader ──────────────────────────────

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

// ── Tone mapping operators ──────────────────────────────────────────

fn reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (color + vec3(1.0));
}

fn aces_fitted(color: vec3<f32>) -> vec3<f32> {
    // ACES filmic tone mapping (Narkowicz 2015)
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return saturate((color * (a * color + b)) / (color * (c * color + d) + e));
}

fn exponential_tonemap(color: vec3<f32>) -> vec3<f32> {
    return vec3(1.0) - exp(-color);
}

fn linear_tonemap(color: vec3<f32>, exposure: f32) -> vec3<f32> {
    return color * exposure;
}

// ── Fragment shader ─────────────────────────────────────────────────

@fragment
fn fs_tonemapping(input: VertexOutput) -> @location(0) vec4<f32> {
    let hdr_color = textureSample(hdr_texture, hdr_sampler, input.uv);
    var mapped: vec3<f32>;

    let exposed = hdr_color.rgb * params.exposure;

    switch (params.operator) {
        case 0u: {
            mapped = reinhard(exposed);
        }
        case 1u: {
            mapped = aces_fitted(exposed);
        }
        case 2u: {
            mapped = exponential_tonemap(exposed);
        }
        case 3u: {
            mapped = linear_tonemap(exposed, 1.0);
        }
        default: {
            mapped = aces_fitted(exposed);
        }
    }

    // Gamma correction
    mapped = pow(mapped, vec3(1.0 / params.gamma));

    return vec4(mapped, hdr_color.a);
}
