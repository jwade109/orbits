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

fn sdf_ring_t(p: vec2<f32>, center: vec2<f32>, r_center: f32, thickness: f32) -> f32 {
    let d = length(p - center) - r_center;
    return abs(d) - thickness;
}

fn sdf_ring_ul(p: vec2<f32>, center: vec2<f32>, r_lower: f32, r_upper: f32) -> f32 {
    let r_center = (r_lower + r_upper) / 2.0;
    let thickness = r_upper - r_lower;
    return sdf_ring_t(p, center, r_center, thickness);
}

fn sdf_ellipse(p: vec2<f32>, center: vec2<f32>, r: vec2<f32>) -> f32 {
    let d = p - center;
    let k = length(d / r);
    return (k - 1.0) * min(r.x, r.y);
}

fn loop_anim(t: f32, dur: f32) -> f32 {
    return fract(t / dur) * dur;
}

fn smin( a: f32, b: f32, k: f32 ) -> f32
{
    let r = exp2(-a/k) + exp2(-b/k);
    return -k*log2(r);
}

fn random(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898,78.233))) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Four corners in 2D of a tile
    let a = random(i);
    let b = random(i + vec2<f32>(1.0, 0.0));
    let c = random(i + vec2<f32>(0.0, 1.0));
    let d = random(i + vec2<f32>(1.0, 1.0));

    // Smooth Interpolation

    // Cubic Hermine Curve.  Same as SmoothStep()
    let u = f*f*(3.0-2.0*f);
    // u = smoothstep(0.,1.,f);

    // Mix 4 coorners percentages
    return mix(a, b, u.x) +
            (c - a)* u.y * (1.0 - u.x) +
            (d - b) * u.x * u.y;
}

fn better_noise(p: vec2<f32>) -> f32
{
    return noise(p * 10.0) * 0.5 +
           noise(p * 500.0) * 0.2 +
           noise(p * 1000.0) * 0.1 +
           noise(p * 2000.0) * 0.1 +
           noise(p * 5000.0) * 0.05;
}

fn sdf_pie(o: vec2<f32>, t: f32, angle: f32, r: f32) -> f32 {
    let c = vec2<f32>(sin(t), cos(t));
    var p = o;
    p.x = abs(p.x);
    p = vec2<f32>(
        p.x * cos(angle) - p.y * sin(angle),
        p.x * sin(angle) + p.y * sin(angle)
    );
    let l = length(p) - r;
    let m = length(p-c*clamp(dot(p,c),0.0,r)); // c=sin/cos of aperture
    return max(l,m*sign(c.y*p.x-c.x*p.y));
}

fn sdf_circle(p: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
    let d = length(p - center);
    return d - radius;
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
