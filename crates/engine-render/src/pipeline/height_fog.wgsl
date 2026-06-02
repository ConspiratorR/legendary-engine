// ── Height Fog ─────────────────────────────────────────────────────
// Exponential height-based fog applied as a post-processing pass.

struct FogParams {
    fog_color: vec3<f32>,
    fog_density: f32,
    fog_height_falloff: f32,
    fog_start_distance: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var scene_texture: texture_2d<f32>;

@group(0) @binding(1)
var position_texture: texture_2d<f32>;

@group(0) @binding(2)
var tex_sampler: sampler;

@group(0) @binding(3)
var<uniform> params: FogParams;

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
fn fs_height_fog(input: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_texture, tex_sampler, input.uv);
    let world_pos = textureSample(position_texture, tex_sampler, input.uv).xyz;

    // Exponential height fog
    let height_factor = exp(-max(world_pos.y, 0.0) * params.fog_height_falloff);
    let distance = length(world_pos);
    let distance_factor = exp(-max(distance - params.fog_start_distance, 0.0) * params.fog_density);
    let fog_factor = saturate(height_factor * (1.0 - distance_factor));

    let result = mix(scene.rgb, params.fog_color, fog_factor);
    return vec4(result, scene.a);
}

fn saturate(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}
