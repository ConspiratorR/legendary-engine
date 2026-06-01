// ── Deferred Geometry Pass ──────────────────────────────────────────
// Outputs to 4 render targets: albedo, normal, position, material

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
}

struct GeometryPushConstants {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

var<push_constant> pc: GeometryPushConstants;

// ── Vertex ──────────────────────────────────────────────────────────

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = pc.model * vec4(input.position, 1.0);
    output.clip_position = camera.view_proj * world_pos;
    output.uv = input.uv;
    output.world_normal = normalize((pc.normal_matrix * vec4(input.normal, 0.0)).xyz);
    output.world_position = world_pos.xyz;
    return output;
}

// ── Fragment ────────────────────────────────────────────────────────

struct GBufferOutput {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) position: vec4<f32>,
    @location(3) material: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> base_color: vec4<f32>;

@group(2) @binding(1)
var<uniform> material_params: vec4<f32>; // metallic, roughness, ao, _pad

@group(2) @binding(2)
var albedo_texture: texture_2d<f32>;

@group(2) @binding(3)
var albedo_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> GBufferOutput {
    var output: GBufferOutput;

    // Albedo: base color RGB + alpha
    let tex_color = textureSample(albedo_texture, albedo_sampler, input.uv);
    output.albedo = base_color * tex_color;

    // Normal: pack from [-1,1] to [0,1]
    let n = normalize(input.world_normal);
    output.normal = vec4(n * 0.5 + 0.5, 1.0);

    // World position
    output.position = vec4(input.world_position, 1.0);

    // Material: metallic R, roughness G, ao B
    output.material = vec4(material_params.x, material_params.y, material_params.z, 1.0);

    return output;
}
