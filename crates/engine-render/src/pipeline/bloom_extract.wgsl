// ── Bloom Brightness Extraction ─────────────────────────────────────
// Extracts pixels above a luminance threshold from the HDR framebuffer.

struct BloomExtractParams {
    threshold: f32,
    soft_knee: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var hdr_texture: texture_2d<f32>;

@group(0) @binding(1)
var tex_sampler: sampler;

@group(0) @binding(2)
var<uniform> params: BloomExtractParams;

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

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

// Soft knee threshold for smooth bloom transition
fn soft_threshold(lum: f32, threshold: f32, knee: f32) -> f32 {
    let x = clamp(lum - threshold + knee, 0.0, 2.0 * knee);
    return x * x / (4.0 * knee + 0.00001);
}

@fragment
fn fs_extract(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(hdr_texture, tex_sampler, input.uv);
    let lum = luminance(color.rgb);

    // Soft threshold for smooth transition
    let contribution = max(soft_threshold(lum, params.threshold, params.soft_knee), lum - params.threshold);
    let brightness = max(contribution, 0.0);

    return vec4(color.rgb * (brightness / max(lum, 0.00001)), 1.0);
}
