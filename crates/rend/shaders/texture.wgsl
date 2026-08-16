// import("common.wgsl")

@group(0) @binding(0) var<uniform> transform: mat4x4<f32>;
@group(1) @binding(0) var the_texture: texture_2d<f32>;
@group(1) @binding(1) var the_sampler: sampler;
@group(2) @binding(0) var<uniform> params: ShaderParams;
@group(3) @binding(0) var<uniform> camera_projection: mat4x4<f32>;
@group(4) @binding(0) var<uniform> light_source: vec3<f32>;

struct ShaderParams {
    mouse_pos: vec2<f32>,
    resolution: vec2<f32>,
    time: f32,
}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
};

struct VertexShaderOut {
    @builtin(position) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(2) world_space_position: vec4<f32>,
};

struct FragmentShaderOut {
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instanceIndex: u32) -> VertexShaderOut {
    var out: VertexShaderOut;
    out.world_space_position = transform * vec4<f32>(vertex.position, 1.0);
    let a = out.world_space_position.x / 32.0 + out.world_space_position.z / 20.0 + params.time / 5.0;
    out.world_space_position.y += 3.0 * sin(a);
    out.position = camera_projection * out.world_space_position;
    out.tex_coord = vec2<f32>(vertex.tex_coord.x, 1.0 - vertex.tex_coord.y);
    out.color = vertex.color;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOut) -> FragmentShaderOut {
    var out: FragmentShaderOut;
    // var c = textureSample(the_texture, the_sampler, in.tex_coord);
    out.color.x = pow(out.color.x, 2.0);
    out.color.y = pow(out.color.y, 2.0);
    out.color.z = pow(out.color.z, 2.0);

    // out.color = c;

    // return out;

    var l = light_source;
    let range = 1.0 + sin(params.time) * 0.2;

    let d = length(l - in.world_space_position.xyz);
    let light_color = vec4<f32>(1.0, 0.8, 0.2, 1.0);
    var light_strength = 0.0;

    if d < 12.0 * range {
        light_strength = 0.2;
    }
    if d < 6.0 * range {
        light_strength = 0.5;
    }
    if d < 3.0 * range {
        light_strength = 1.0;
    }

    out.color = mix(in.color, light_color, light_strength);

    let y = in.world_space_position.y;
    let fade_out_color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    let fade_out_magnitude = round(smoothstep(4.0, -12.0, y) * 20.0) / 20.0;

    out.color = mix(in.color, fade_out_color, fade_out_magnitude);

    return out;
}
