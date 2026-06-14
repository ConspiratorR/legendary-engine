// ── Deferred Lighting Pass ──────────────────────────────────────────
// Full-screen triangle vertex shader + PBR lighting fragment shader

struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
}

// Multi-light packed lighting data (using vec4 for 16-byte alignment)
struct LightingData {
    ambient: vec4<f32>,       // xyz = color, w = intensity
    dir_count: u32,
    point_count: u32,
    spot_count: u32,
    _pad: u32,
    // Up to 4 directional lights: each is [dir.xyz, _pad, color.xyz, _]
    directional: array<vec4<f32>, 8>,
    // Up to 16 point lights: each is [pos.xyz, range, color.xyz, _]
    point_lights: array<vec4<f32>, 32>,
    // Up to 8 spot lights: each is [pos.xyz, range, dir.xyz, intensity, inner, outer, _, _]
    spot: array<vec4<f32>, 24>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> lighting: LightingData;

// G-Buffer textures
@group(2) @binding(0)
var gbuffer_albedo: texture_2d<f32>;

@group(2) @binding(1)
var gbuffer_normal: texture_2d<f32>;

@group(2) @binding(2)
var gbuffer_position: texture_2d<f32>;

@group(2) @binding(3)
var gbuffer_material: texture_2d<f32>;

@group(2) @binding(4)
var gbuffer_sampler: sampler;

// Shadow mapping
struct ShadowUniform {
    light_vp: mat4x4<f32>,
    shadow_bias: f32,
    normal_bias: f32,
    cascade_count: u32,
    _pad: f32,
}

@group(3) @binding(0)
var shadow_map: texture_depth_2d;

@group(3) @binding(1)
var shadow_sampler: sampler_comparison;

@group(3) @binding(2)
var<uniform> shadow: ShadowUniform;

// ── Full-screen triangle vertex shader ──────────────────────────────

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Generate a full-screen triangle (3 vertices, no vertex buffer)
    var output: VertexOutput;
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    output.clip_position = vec4(x, y, 0.0, 1.0);
    // UV: (0,0) at bottom-left, (1,1) at top-right
    output.uv = vec2((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return output;
}

// ── PBR lighting (same model as pbr.wgsl) ───────────────────────────

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

fn sample_shadow(world_position: vec3<f32>, normal: vec3<f32>, light_dir: vec3<f32>) -> f32 {
    let light_pos = shadow.light_vp * vec4(world_position, 1.0);
    let ndc = light_pos.xyz / light_pos.w;
    let shadow_uv = vec2(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let shadow_depth = ndc.z;

    let n_dot_l = saturate(dot(normal, light_dir));
    let bias = shadow.shadow_bias + shadow.normal_bias * (1.0 - n_dot_l);

    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 || shadow_uv.y < 0.0 || shadow_uv.y > 1.0 {
        return 1.0;
    }
    if shadow_depth < 0.0 || shadow_depth > 1.0 {
        return 1.0;
    }

    let texel_size = 1.0 / 2048.0;
    let offsets = array<vec2<f32>, 5>(
        vec2(0.0, 0.0),
        vec2(-1.0, 0.0) * texel_size,
        vec2(1.0, 0.0) * texel_size,
        vec2(0.0, -1.0) * texel_size,
        vec2(0.0, 1.0) * texel_size,
    );

    var shadow_acc = 0.0;
    for (var i = 0u; i < 5u; i++) {
        let uv = shadow_uv + offsets[i];
        shadow_acc += textureSampleCompare(shadow_map, shadow_sampler, uv, shadow_depth - bias);
    }

    return shadow_acc / 5.0;
}

@fragment
fn fs_lighting(input: VertexOutput) -> @location(0) vec4<f32> {
    // Sample G-Buffer
    let albedo = textureSample(gbuffer_albedo, gbuffer_sampler, input.uv);
    let normal_sample = textureSample(gbuffer_normal, gbuffer_sampler, input.uv);
    let position = textureSample(gbuffer_position, gbuffer_sampler, input.uv);
    let material = textureSample(gbuffer_material, gbuffer_sampler, input.uv);

    // Decode normal from [0,1] back to [-1,1]
    let normal = normalize(normal_sample.xyz * 2.0 - 1.0);
    let world_position = position.xyz;

    let base_color = albedo.rgb;
    let metallic = material.r;
    let roughness = max(material.g, 0.04);
    let ao = material.b;
    let alpha = albedo.a;

    let view_dir = normalize(camera.camera_pos - world_position);

    // Start with ambient
    var total_light = lighting.ambient.xyz * lighting.ambient.w * base_color * ao;

    // ── Directional lights ──
    for (var i = 0u; i < lighting.dir_count; i++) {
        let base = i * 2u;
        let ld0 = lighting.directional[base];
        let ld1 = lighting.directional[base + 1u];
        let light_dir = normalize(-ld0.xyz);
        let light_color = ld1.xyz;

        let ndotl = saturate(dot(normal, light_dir));
        let diffuse = light_color * ndotl * base_color;

        let half_dir = normalize(light_dir + view_dir);
        let ndoth = saturate(dot(normal, half_dir));
        let spec = pow(ndoth, mix(8.0, 256.0, 1.0 - roughness));
        let specular = light_color * spec * mix(0.04, 1.0, metallic);

        // Shadow attenuation (first directional light only)
        var shadow_factor = 1.0;
        if i == 0u {
            shadow_factor = sample_shadow(world_position, normal, light_dir);
        }

        total_light += (diffuse + specular) * shadow_factor;
    }

    // ── Point lights ──
    for (var i = 0u; i < lighting.point_count; i++) {
        let base = i * 2u;
        let pd0 = lighting.point_lights[base];
        let pd1 = lighting.point_lights[base + 1u];
        let light_pos = pd0.xyz;
        let range = pd0.w;
        let light_color = pd1.xyz;

        let to_light = light_pos - world_position;
        let dist = length(to_light);
        let light_dir = to_light / max(dist, 0.001);

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
        let base = i * 3u;
        let sd0 = lighting.spot[base];
        let sd1 = lighting.spot[base + 1u];
        let sd2 = lighting.spot[base + 2u];
        let light_pos = sd0.xyz;
        let range = sd0.w;
        let spot_dir = normalize(sd1.xyz);
        let intensity = sd1.w;
        let inner = sd2.x;
        let outer = sd2.y;

        let to_light = light_pos - world_position;
        let dist = length(to_light);
        let light_dir = to_light / max(dist, 0.001);

        let attenuation = saturate(1.0 - (dist * dist) / (range * range));
        let att = attenuation * attenuation;

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

    return vec4(total_light, alpha);
}
