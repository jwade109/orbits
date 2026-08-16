@group(0) @binding(0) var<uniform> line_data: array<LineData, MAX_LINES_PER_PASS>;

struct LineData {
    position: vec4<f32>,
    color: vec4<f32>,
    thickness: f32,
    screen_width: f32,
    screen_height: f32,
    _pad2: f32,
};

const MAX_LINES_PER_PASS: u32 = 600;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) instance_index: u32,
    @location(1) uv: vec2<f32>,
    @location(2) length: f32,
    @location(3) width: f32,
};

fn rotate_vector(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let cs = cos(angle);
    let sn = sin(angle);
    return vec2<f32>(p.x * cs - p.y * sn, p.x * sn + p.y * cs);
}

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    let data = line_data[vertex.instance_index];

    var start = data.position.xy;
    var end = data.position.zw;

    let w = data.screen_width;
    let h = data.screen_height;
    let t = data.thickness / 2.0;

    out.length = length(end - start);
    out.width = data.thickness;

    let line_angle = atan2(end.y - start.y, end.x - start.x);
    let end_cap_offset = vec2<f32>(t, t);

    let a = start + rotate_vector(vec2<f32>(-t,  t), line_angle);
    let b = start + rotate_vector(vec2<f32>(-t, -t), line_angle);
    let c = end   + rotate_vector(vec2<f32>( t, -t), line_angle);
    let d = end   + rotate_vector(vec2<f32>( t,  t), line_angle);

    let dims = vec2<f32>(w, h);

    let positions = array<vec2<f32>, 4>(
        d / dims,
        c / dims,
        b / dims,
        a / dims,
    );

    var pos = positions[vertex.vertex_index] * 2.0 - 1.0;
    pos.y *= -1.0;
    out.position = vec4<f32>(pos, 1.0, 1.0);
    out.instance_index = vertex.instance_index;
    out.uv = vertex.uv;
    return out;
}

fn sdf_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    var data = line_data[in.instance_index];

    data.color.r = pow(data.color.r, 2.2);
    data.color.g = pow(data.color.g, 2.2);
    data.color.b = pow(data.color.b, 2.2);

    let t = data.thickness / 2.0;

    let a = vec2<f32>(t, t);
    let b = vec2<f32>(t, in.length - t);

    let p = in.uv * vec2<f32>(in.width, in.length);

    let d = sdf_segment(p, a, b);

    let alpha = 1.0 - smoothstep(t - 2.0, t, d);
    data.color.a *= alpha;

    // let x = in.uv.x * in.width;
    // let y = in.uv.y * in.length;
    // let p = vec2<f32>(x, y) - vec2<f32>(in.width, in.length) / 2.0;

    // let start = data.position.xy;
    // let end = data.position.zw;

    // let dist = sdf_box(p, vec2<f32>(in.width, in.length) / 2.0);

    // let alpha = 1.0 - smoothstep(-3.0, -1.0, dist);
    // data.color.a *= alpha;

    return data.color; // vec4<f32>(in.uv, 0.0, 1.0);
}
