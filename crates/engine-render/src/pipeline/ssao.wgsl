// ── Screen Space Ambient Occlusion (SSAO) ──────────────────────────
// Full-screen pass that computes ambient occlusion from G-Buffer data.
// Uses hemisphere kernel sampling with cosine-weighted distribution.

struct SsaoParams {
    kernel_size: u32,
    radius: f32,
    bias: f32,
    intensity: f32,
    noise_scale: vec2<f32>,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0)
var position_texture: texture_2d<f32>;

@group(0) @binding(1)
var normal_texture: texture_2d<f32>;

@group(0) @binding(2)
var noise_texture: texture_2d<f32>;

@group(0) @binding(3)
var tex_sampler: sampler;

@group(0) @binding(4)
var<uniform> params: SsaoParams;

// Hemisphere kernel (up to 64 samples)
@group(0) @binding(5)
var<storage, read> kernel: array<vec4<f32>, 64>;

// View-projection matrix for reconstructing screen-space positions
@group(1) @binding(0)
var<uniform> view_proj: mat4x4<f32>;

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
fn fs_ssao(input: VertexOutput) -> @location(0) f32 {
    let frag_pos = textureSample(position_texture, tex_sampler, input.uv).xyz;
    let normal = normalize(textureSample(normal_texture, tex_sampler, input.uv).xyz * 2.0 - 1.0);

    // Sample noise for rotation
    let noise = textureSample(noise_texture, tex_sampler, input.uv * params.noise_scale).xy;

    // Construct TBN matrix from normal + random rotation
    let tangent = normalize(noise.x * normal + vec3(0.0, 0.0, 1.0));
    let bitangent = cross(normal, tangent);
    let tbn = mat3x3<f32>(tangent, bitangent, normal);

    var occlusion = 0.0;
    for (var i = 0u; i < params.kernel_size; i++) {
        // Rotate sample by TBN
        let sample_dir = tbn * kernel[i].xyz;
        let sample_pos = frag_pos + sample_dir * params.radius;

        // Project sample to screen space
        let projected = view_proj * vec4(sample_pos, 1.0);
        let ndc = projected.xyz / projected.w;
        let sample_uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);

        // Sample depth at projected position
        let sample_depth = textureSample(position_texture, tex_sampler, sample_uv).z;

        // Range check and occlusion test
        let range_check = smoothstep(0.0, 1.0, params.radius / abs(frag_pos.z - sample_depth));
        occlusion += select(0.0, 1.0, sample_depth >= sample_pos.z + params.bias) * range_check;
    }

    occlusion = 1.0 - (occlusion / f32(params.kernel_size)) * params.intensity;
    return occlusion;
}
