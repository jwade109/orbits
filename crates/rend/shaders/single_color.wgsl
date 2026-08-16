@group(0) @binding(0) var<uniform> transform: mat4x4<f32>;
@group(1) @binding(0) var<uniform> color: vec4<f32>;

struct Vertex {
    @location(0) position: vec3<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    out.position = transform * vec4<f32>(vertex.position, 1.0);
    return out;
}

fn srgb_to_linear(c: vec4<f32>) -> vec4<f32> {
    var out = c;
    out.x = pow(out.x, 2.0);
    out.y = pow(out.y, 2.0);
    out.z = pow(out.z, 2.0);
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    return srgb_to_linear(color);
}
