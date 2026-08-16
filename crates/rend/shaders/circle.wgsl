@group(0) @binding(0) var<uniform> color_array: array<vec4<f32>, MAX_CHARS_PER_PASS>;
@group(1) @binding(0) var<uniform> transforms_array: array<mat4x4<f32>, MAX_CHARS_PER_PASS>;

// only the 0th field of the vec4 is used. the rest is for padding
@group(2) @binding(0) var<uniform> radius_array: array<vec4<f32>, MAX_CHARS_PER_PASS>;

const MAX_CHARS_PER_PASS: u32 = 480;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) instance_index: u32,
    @location(1) pixels: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    out.position = transforms_array[vertex.instance_index] * vec4<f32>(vertex.position, 1.0);
    out.instance_index = vertex.instance_index;
    let r = radius_array[out.instance_index].x;
    out.pixels = (vertex.uv * 2.0 - 1.0) * r;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    var color = color_array[in.instance_index];
    let r = radius_array[in.instance_index].x;

    color.r = pow(color.r, 2.2);
    color.g = pow(color.g, 2.2);
    color.b = pow(color.b, 2.2);

    color.a *= (1.0 - smoothstep(r - 2.0, r, length(in.pixels)));
    return color;
}
