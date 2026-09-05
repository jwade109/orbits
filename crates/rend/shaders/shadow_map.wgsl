// import("common.wgsl")

@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var sample: sampler;
@group(1) @binding(0) var<uniform> params: ShaderParams;

struct ShaderParams {
    mouse_pos: vec2<f32>,
    resolution: vec2<f32>,
    time: f32,
}

struct Vertex {
    @location(0) position: vec3<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    out.position = vec4<f32>(vertex.position, 1.0);
    return out;
}

fn hill(p: vec2<f32>, peak: vec2<f32>, height: f32) -> f32 {
    let d = length(p - peak);
    let z = height / (1.0 + d / height);
    return z;
}

fn range(p: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, height: f32) -> f32 {
    let d = sdf_segment(p, p1, p2);
    let z = height / (1.0 + d / height);
    return z;
}

fn sinusoid(p: vec2<f32>) -> f32 {
    return sin(p.x);
}

const p1 = vec2<f32>(700.0, 800.0);
const p2 = vec2<f32>(1500.0, 1200.0);
const p3 = vec2<f32>(1700.0, 600.0);

fn height_func(p: vec2<f32>) -> f32 {
    let uv = p / params.resolution;
    let l = length(textureSample(texture, sample, uv).xyz);
    return 1.0 - l;
}

fn is_in_shadow(pz: vec3<f32>, sun: vec3<f32>) -> bool {
    var sample = pz;
    let u = normalize(sun - sample);

    var i = 0;

    let n = 300;
    let dist = length(pz.xy - sun.xy);
    let step_size = dist / f32(n);

    while (length(sample - sun) > 5.0)
    {
        i += 1;
        if (i > n)
        {
            break;
        }

        sample += u * step_size;
        let z_sample = height_func(sample.xy);
        if (z_sample > sample.z) {
            return true;
        }
        if (sample.z > 1.0) {
            return false;
        }
    }

    return false;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {

    let sun = vec3<f32>(params.mouse_pos, 12.0 + sin(params.time));

    // let pix = 1.0;
    let p = in.position.xy; // floor(in.position.xy / pix) * pix;

    let color_here = textureSample(texture, sample, p / params.resolution);

    let z = height_func(p);

    let pz = vec3<f32>(p, z);

    let is_in_shadow = is_in_shadow(pz, sun);

    var t = 0.0;

    if (is_in_shadow)
    {
        t = 0.3;
    }

    return mix(color_here, vec4<f32>(0.0, 0.0, 0.0, 1.0), t);
}
