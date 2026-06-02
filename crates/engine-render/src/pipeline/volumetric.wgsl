// ── Volumetric Light ───────────────────────────────────────────────
// Screen-space volumetric lighting via ray marching through the depth buffer.
// Samples along view rays and accumulates light contribution.

struct VolumetricParams {
    light_pos: vec3<f32>,
    scattering: f32,
    max_distance: f32,
    num_steps: u32,
    intensity: f32,
    _pad: f32,
};

@group(0) @binding(0)
var depth_texture: texture_2d<f32>;

@group(0) @binding(1)
var position_texture: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

@group(0) @binding(3)
var<uniform> params: VolumetricParams;

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

fn henvey_greenstein(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    return (1.0 - g2) / (4.0 * 3.14159 * pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5));
}

@fragment
fn fs_volumetric(input: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = textureSample(position_texture, tex_sampler, input.uv).xyz;
    let depth = textureSample(depth_texture, tex_sampler, input.uv).r;

    // Simple volumetric light accumulation
    let step_size = params.max_distance / f32(params.num_steps);
    var accumulation = 0.0;

    for (var i = 0u; i < params.num_steps; i++) {
        let t = f32(i) * step_size;
        // Sample along view ray (simplified: assumes forward-facing camera)
        let sample_pos = world_pos + vec3(0.0, 0.0, -t);

        // Check if sample is behind scene geometry
        let sample_depth = textureSample(depth_texture, tex_sampler, input.uv).r;
        if (sample_depth < t) {
            break;
        }

        // Scattering contribution
        let to_light = params.light_pos - sample_pos;
        let dist_to_light = length(to_light);
        let cos_theta = dot(normalize(to_light), vec3(0.0, 0.0, -1.0));
        let phase = henvey_greenstein(cos_theta, params.scattering);

        accumulation += phase / max(dist_to_light * dist_to_light, 1.0);
    }

    accumulation *= step_size * params.intensity;
    return vec4(vec3(accumulation), 1.0);
}
