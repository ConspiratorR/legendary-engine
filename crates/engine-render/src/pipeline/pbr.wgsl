// ── Uniforms ────────────────────────────────────────────────────────

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
}

// Multi-light packed lighting data
struct LightingData {
    ambient: vec4<f32>,       // xyz = color, w = intensity
    dir_count: u32,
    point_count: u32,
    spot_count: u32,
    _pad: u32,
    // Up to 4 directional lights: each is [dir.xyz, _pad, color.xyz, _]
    directional: array<array<f32, 8>, 4>,
    // Up to 16 point lights: each is [pos.xyz, range, color.xyz, _]
    point_lights: array<array<f32, 8>, 16>,
    // Up to 8 spot lights: each is [pos.xyz, range, dir.xyz, intensity, inner, outer, _, _]
    spot: array<array<f32, 12>, 8>,
}

struct MaterialData {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    ao: f32,
    _pad0: f32,
    emissive: vec3<f32>,
    _pad1: f32,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> lighting: LightingData;

@group(2) @binding(0)
var<uniform> material: MaterialData;

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

var<push_constant> model: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = model * vec4(input.position, 1.0);
    output.clip_position = camera.view_proj * world_pos;
    output.uv = input.uv;
    output.world_normal = normalize((model * vec4(input.normal, 0.0)).xyz);
    output.world_position = world_pos.xyz;
    return output;
}

// ── Fragment ────────────────────────────────────────────────────────

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    let view_dir = normalize(camera.camera_pos - input.world_position);

    let base_color = material.base_color.rgb;
    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04);

    // Start with ambient
    var total_light = lighting.ambient.xyz * lighting.ambient.w * base_color * material.ao;

    // ── Directional lights ──
    for (var i = 0u; i < lighting.dir_count; i++) {
        let ld = lighting.directional[i];
        let light_dir = normalize(-vec3(ld[0], ld[1], ld[2]));
        let light_color = vec3(ld[4], ld[5], ld[6]);

        let ndotl = saturate(dot(normal, light_dir));
        let diffuse = light_color * ndotl * base_color;

        // Blinn-Phong specular
        let half_dir = normalize(light_dir + view_dir);
        let ndoth = saturate(dot(normal, half_dir));
        let spec = pow(ndoth, mix(8.0, 256.0, 1.0 - roughness));
        let specular = light_color * spec * mix(0.04, 1.0, metallic);

        total_light += diffuse + specular;
    }

    // ── Point lights ──
    for (var i = 0u; i < lighting.point_count; i++) {
        let pd = lighting.point_lights[i];
        let light_pos = vec3(pd[0], pd[1], pd[2]);
        let range = pd[3];
        let light_color = vec3(pd[4], pd[5], pd[6]);

        let to_light = light_pos - input.world_position;
        let dist = length(to_light);
        let light_dir = to_light / max(dist, 0.001);

        // Smooth distance attenuation
        let attenuation = saturate(1.0 - (dist * dist) / (range * range));
        let att = attenuation * attenuation;

        let ndotl = saturate(dot(normal, light_dir));
        let diffuse = light_color * ndotl * base_color * att;

        let half_dir = normalize(light_dir + view_dir);
        let ndoth = saturate(dot(normal, half_dir));
        let spec = pow(ndoth, mix(8.0, 256.0, 1.0 - roughness));
        let specular = light_color * spec * mix(0.04, 1.0, metallic) * att;

        total_light += diffuse + specular;
    }

    // ── Spot lights ──
    for (var i = 0u; i < lighting.spot_count; i++) {
        let sd = lighting.spot[i];
        let light_pos = vec3(sd[0], sd[1], sd[2]);
        let range = sd[3];
        let spot_dir = normalize(vec3(sd[4], sd[5], sd[6]));
        let intensity = sd[7];
        let inner = sd[8];
        let outer = sd[9];

        let to_light = light_pos - input.world_position;
        let dist = length(to_light);
        let light_dir = to_light / max(dist, 0.001);

        let attenuation = saturate(1.0 - (dist * dist) / (range * range));
        let att = attenuation * attenuation;

        // Spot cone falloff
        let cos_angle = dot(-light_dir, spot_dir);
        let spot_att = saturate((cos_angle - cos(outer)) / (cos(inner) - cos(outer)));

        let ndotl = saturate(dot(normal, light_dir));
        let diffuse = vec3(intensity) * ndotl * base_color * att * spot_att;

        let half_dir = normalize(light_dir + view_dir);
        let ndoth = saturate(dot(normal, half_dir));
        let spec = pow(ndoth, mix(8.0, 256.0, 1.0 - roughness));
        let specular = vec3(intensity) * spec * mix(0.04, 1.0, metallic) * att * spot_att;

        total_light += diffuse + specular;
    }

    // Emissive
    total_light += material.emissive;

    return vec4(total_light, material.base_color.a);
}
