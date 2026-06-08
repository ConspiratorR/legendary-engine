struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
};

struct ShapeUniform {
    transform: mat4x4<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
    stroke_color: vec4<f32>,
    stroke_width: f32,
    corner_radius: f32,
    shape_type: u32,
    _padding: u32,
};

@group(0) @binding(0)
var<uniform> shape: ShapeUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = shape.transform * vec4<f32>(in.position, 0.0, 1.0);
    out.local_pos = in.position;
    return out;
}

fn sdBox(p: vec2<f32>, b: vec2<f32>) -> f32 {
    let d = abs(p) - b;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

fn sdCircle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sdEllipse(p: vec2<f32>, ab: vec2<f32>) -> f32 {
    let pa = p / ab;
    return (length(pa) - 1.0) * min(ab.x, ab.y);
}

fn sdRoundedBox(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sdSegment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.local_pos;
    var d: f32;

    switch shape.shape_type {
        case 0u: { d = sdBox(p, shape.size * 0.5); }
        case 1u: { d = sdCircle(p, shape.size.x * 0.5); }
        case 2u: { d = sdEllipse(p, shape.size * 0.5); }
        case 3u: { d = sdRoundedBox(p, shape.size * 0.5, shape.corner_radius); }
        case 4u: { d = sdSegment(p, vec2<f32>(-0.5, 0.0), vec2<f32>(0.5, 0.0)) - shape.size.y * 0.5; }
        default: { d = 1.0; }
    }

    let aa = 1.0;
    let fill_alpha = 1.0 - smoothstep(-aa, aa, d);
    var color = vec4<f32>(shape.color.rgb, shape.color.a * fill_alpha);

    if shape.stroke_width > 0.0 {
        let stroke_d = abs(d) - shape.stroke_width;
        let stroke_alpha = 1.0 - smoothstep(-aa, aa, stroke_d);
        let stroke_mix = shape.stroke_color.a * stroke_alpha;
        color = mix(color, shape.stroke_color, stroke_mix);
    }

    return color;
}
