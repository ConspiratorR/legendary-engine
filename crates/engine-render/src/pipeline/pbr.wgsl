struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
}

struct LightUniform {
    direction: vec3<f32>,
    color: vec3<f32>,
    ambient: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: LightUniform;

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

// Push constant: model matrix (64 bytes)
var<push_constant> model: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = model * vec4(input.position, 1.0);
    output.clip_position = camera.view_proj * world_pos;
    output.uv = input.uv;
    // Normal transform (assuming uniform scale for simplicity)
    output.world_normal = normalize((model * vec4(input.normal, 0.0)).xyz);
    output.world_position = world_pos.xyz;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    let light_dir = normalize(-light.direction);

    // Diffuse (Lambertian)
    let ndotl = max(dot(normal, light_dir), 0.0);
    let diffuse = light.color * ndotl;

    // Ambient
    let ambient = light.color * light.ambient;

    // Simple specular (Blinn-Phong for now)
    let view_dir = normalize(camera.camera_pos - input.world_position);
    let half_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(normal, half_dir), 0.0), 32.0) * 0.3;
    let specular = light.color * spec;

    // Base color (white — material system will override later)
    let base_color = vec3(0.8, 0.8, 0.8);

    let final_color = base_color * (ambient + diffuse) + specular;
    return vec4(final_color, 1.0);
}
