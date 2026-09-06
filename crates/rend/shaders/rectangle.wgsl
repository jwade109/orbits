@group(0) @binding(0) var<uniform> rect_data: array<RectData, RECTS_PER_PASS>;

const RECTS_PER_PASS: u32 = 1300;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    let data = rect_data[vertex.instance_index];

    let w = data.screen_x;
    let h = data.screen_y;

    let corners = rect_corners(data.pos, data.dims, data.angle);

    let dims = vec2<f32>(w, h);

    var pos = corners[vertex.vertex_index] / dims * 2.0 - 1.0;
    out.position = vec4<f32>(pos, data.z, 1.0);
    out.color = vec4f(data.r, data.g, data.b, data.a);
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    return in.color;
}
