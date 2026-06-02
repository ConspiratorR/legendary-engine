// ── Screen Space Reflections (SSR) ─────────────────────────────────
// Hierarchical ray marching against the depth buffer.
// Produces reflection color + validity mask.

struct SsrParams {
    max_steps: u32,
    max_distance: f32,
    thickness: f32,
    stride: f32,
    _pad: vec3<f32>,
};

@group(0) @binding(0)
var color_texture: texture_2d<f32>;

@group(0) @binding(1)
var depth_texture: texture_2d<f32>;

@group(0) @binding(2)
var normal_texture: texture_2d<f32>;

@group(0) @binding(3)
var position_texture: texture_2d<f32>;

@group(0) @binding(4)
var tex_sampler: sampler;

@group(0) @binding(5)
var<uniform> params: SsrParams;

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

struct SsrOutput {
    @location(0) reflection: vec4<f32>,
    @location(1) mask: vec4<f32>,
};

@fragment
fn fs_ssr(input: VertexOutput) -> SsrOutput {
    let world_pos = textureSample(position_texture, tex_sampler, input.uv).xyz;
    let normal = normalize(textureSample(normal_texture, tex_sampler, input.uv).xyz * 2.0 - 1.0);
    let depth = textureSample(depth_texture, tex_sampler, input.uv).r;

    // Skip sky pixels
    if (depth >= 1.0) {
        var out: SsrOutput;
        out.reflection = vec4(0.0);
        out.mask = vec4(0.0);
        return out;
    }

    // Compute reflection direction (simplified: assumes view direction along -Z)
    let view_dir = vec3(0.0, 0.0, -1.0);
    let reflect_dir = reflect(view_dir, normal);

    // Ray march in screen space
    let texel_size = 1.0 / vec2<f32>(textureDimensions(depth_texture));
    var current_uv = input.uv;
    var hit = false;

    for (var i = 0u; i < params.max_steps; i++) {
        let step = reflect_dir * params.stride * f32(i);
        let sample_uv = input.uv + step.xy * texel_size;

        // Out of bounds check
        if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
            break;
        }

        let sample_depth = textureSample(depth_texture, tex_sampler, sample_uv).r;
        let expected_depth = depth + step.z * params.max_distance;

        // Depth comparison with thickness tolerance
        if (sample_depth < expected_depth && expected_depth - sample_depth < params.thickness) {
            current_uv = sample_uv;
            hit = true;
            break;
        }
    }

    var out: SsrOutput;
    if (hit) {
        out.reflection = textureSample(color_texture, tex_sampler, current_uv);
        out.mask = vec4(1.0);
    } else {
        out.reflection = vec4(0.0);
        out.mask = vec4(0.0);
    }
    return out;
}
