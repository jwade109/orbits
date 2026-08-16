@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var sample: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    out.position = vec4<f32>(vertex.position, 1.0);
    out.tex_coord = vertex.tex_coord;
    out.tex_coord.y = 1.0 - out.tex_coord.y;
    return out;
}

fn do_blur(in: VertexShaderOutput) -> vec4<f32> {

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    // // Example simple 5-tap horizontal offset weights
    const weights = array<f32, 3>(0.227027, 0.316216, 0.070270);
    // color += textureSample(texture, sample, in.tex_coord) * weights[0];

    var uv = (in.tex_coord * 1.0) / 1.0;
    let off = 0.002;
    let n = 5;
    let w = 1.0 / f32(n * 2);

    // uv.x = pow(uv.x, 2.0);
    // uv.y = pow(uv.y, 2.0);

    // color = textureSample(texture, sample, uv);

    for (var i = 0; i < n; i += 1) {
        let offset = vec2<f32>(off * f32(i), 0.0);
        color += textureSample(texture, sample, uv + offset) * w;
        color += textureSample(texture, sample, uv - offset) * w;
    }

    // for (var i = 0; i < n; i += 1) {
    //     let offset = vec2<f32>(0.0, off * f32(i));
    //     color += textureSample(texture, sample, uv + offset) * w;
    //     color += textureSample(texture, sample, uv - offset) * w;
    // }

    color.w = 1.0;

    return color;
}

fn pixelate(in: VertexShaderOutput, n: f32) -> vec4<f32> {
    let uv = round(in.tex_coord * n) / n;
    return textureSample(texture, sample, uv);
}

fn desaturate(in: VertexShaderOutput) -> vec4<f32> {
    let color = textureSample(texture, sample, in.tex_coord);
    return vec4<f32>(color.xyz * 0.3, color.a);
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    // return desaturate(in);
    return pixelate(in, 200.0);
    // return do_blur(in);
}
