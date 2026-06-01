// Shadow map depth-only vertex shader.
//
// Transforms vertices by light_vp * model to produce depth from the
// light's perspective. No fragment shader is needed for a depth-only pass.

var<push_constant> light_vp_model: mat4x4<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> @builtin(position) vec4<f32> {
    return light_vp_model * vec4(input.position, 1.0);
}
