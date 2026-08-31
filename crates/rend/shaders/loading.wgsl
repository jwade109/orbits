// import("common.wgsl")

@group(0) @binding(0) var<uniform> params: ShaderParams;

struct ShaderParams {
    mouse_pos: vec2<f32>,
    resolution: vec2<f32>,
    time: f32,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    out.position = vec4<f32>(vertex.position, 1.0);
    out.color = vertex.color;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {

    var uv = (in.position.xy * 2.0 - params.resolution) / params.resolution.y;

    let t = 3.0*(0.5+0.5*cos(params.time * 0.52));

    let r = 0.3;
    let d = sdf_pie(uv, 0.3, params.time, r);

    var col = vec3<f32>(0.65,0.85,1.);
    if (d > 0) { col = vec3<f32>(0.9,0.6,0.3); }
	col *= 1.0 - exp(-8.0*abs(d));
	col *= 0.8 + 0.2*cos(128.0*abs(d));
	col = mix( col, vec3(1.0), 1.0-smoothstep(0.0,0.015,abs(d)) );
    col *= (1.0 / (1.0 + abs(d) * 100.0));

    return vec4<f32>(col, 1.0);
}
