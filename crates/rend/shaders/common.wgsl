fn discretize(val: f32, levels: f32) -> f32 {
    return round(val * levels) / levels;
}

fn rotate_vector(p: vec2<f32>, angle: f32) -> vec2<f32> {
    let cs = cos(angle);
    let sn = sin(angle);
    return vec2<f32>(p.x * cs - p.y * sn, p.x * sn + p.y * cs);
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

fn rand(x: f32) -> f32 {
    let v = vec2<f32>(x, x);
    return fract(sin(dot(v, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn rgb_to_vec3(r: u32, g: u32, b: u32) -> vec3<f32>
{
    let rf = pow(f32(r) / 255.0, 2.2);
    let gf = pow(f32(g) / 255.0, 2.2);
    let bf = pow(f32(b) / 255.0, 2.2);
    return vec3<f32>(rf, gf, bf);
}

fn sdf_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba,ba), 0.0, 1.0);
    return length(pa - ba*h);
}

fn sdf_capsule(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, padding: f32) -> f32 {
    let r = sdf_segment(p, a, b);
    return r - padding;
}

fn lerp(a: vec4f, b: vec4f, t: f32) -> vec4f {
    return a + (b - a) * t;
}

struct RectData {
    pos:      vec2f,
    dims:     vec2f,
    r:        f32,
    g:        f32,
    b:        f32,
    a:        f32,
    angle:    f32,
    screen_x: f32,
    screen_y: f32,
    _padding: f32,
}
