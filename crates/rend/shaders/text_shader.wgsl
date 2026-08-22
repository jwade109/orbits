@group(0) @binding(0) var the_texture: texture_2d<f32>;
@group(0) @binding(1) var the_sampler: sampler;
@group(1) @binding(0) var<uniform> color_array: array<vec4<f32>, MAX_CHARS_PER_PASS>;
@group(2) @binding(0) var<uniform> sample_info_array: array<SampleInfo, MAX_CHARS_PER_PASS>;
@group(3) @binding(0) var<uniform> transforms_array: array<mat4x4<f32>, MAX_CHARS_PER_PASS>;

const MAX_CHARS_PER_PASS: u32 = 480;

struct SampleInfo {
    origin_x: u32,
    origin_y: u32,
    sample_width: u32,
    sample_height: u32,
    image_width: u32,
    image_height: u32,
    _pad1: u32,
    _pad2: u32,
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) instance_index: u32,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;

    let sample_data = sample_info_array[vertex.instance_index];

    out.position = transforms_array[vertex.instance_index] * vec4<f32>(vertex.position, 1.0);
    out.tex_coord = vertex.tex_coord;
    out.tex_coord.y = 1.0 - out.tex_coord.y;
    out.instance_index = vertex.instance_index;

    let start_x = f32(sample_data.origin_x) / f32(sample_data.image_width);
    let start_y = f32(sample_data.origin_y) / f32(sample_data.image_height);

    let dims_x = f32(sample_data.sample_width) / f32(sample_data.image_width);
    let dims_y = f32(sample_data.sample_height) / f32(sample_data.image_height);

    out.tex_coord.x = start_x + dims_x * out.tex_coord.x;
    out.tex_coord.y = start_y + dims_y * out.tex_coord.y;

    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    var c = textureSample(the_texture, the_sampler, in.tex_coord);
    c.x = pow(c.x, 2.0);
    c.y = pow(c.y, 2.0);
    c.z = pow(c.z, 2.0);

    let l = c.x;

    let col = color_array[in.instance_index];

    // for debugging
    if l < 0.03 {
        return vec4<f32>(1.0, 0.0, 0.0, 0.3);
    }

    return vec4<f32>(col.xyz, col.w * smoothstep(0.14, 0.21, l));
}