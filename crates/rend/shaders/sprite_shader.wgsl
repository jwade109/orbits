@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var sample: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;

    let dx = 0.3;
    let dy = 0.3;

    let pos = vec2f(0.1, 0.1);

    let a = pos;
    let b = pos + rotate_vector(vec2<f32>(dx,  0.0), 0.1);
    let c = pos + rotate_vector(vec2<f32>(dx,  dy),  0.1);
    let d = pos + rotate_vector(vec2<f32>(0.0, dy),  0.1);

    let positions = array<vec2<f32>, 4>(
        a,
        b,
        c,
        d,
    );

    let uvs = array<vec2<f32>, 4>(
        vec2f(0.0, 1.0),
        vec2f(1.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 0.0),
    );

    var v = positions[vertex.vertex_index] * 2.0 - 1.0;
    out.position = vec4<f32>(v, 1.0, 1.0);
    out.uv = uvs[vertex.vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, sample, in.uv);
}
