// import("common.wgsl")

@group(0) @binding(0) var<uniform> uniform_data: ShaderParams;
@group(1) @binding(0) var<uniform> camera_projection: mat4x4<f32>;

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
    out.position = camera_projection * vec4<f32>(vertex.position, 1.0);
    out.color = vertex.color;
    return out;
}

fn get_lava_lamp_color(in: VertexShaderOutput) -> vec4<f32> {

    let PURPLE =      rgb_to_vec3(87u,  16u,  110u); // rgb(87, 16, 110);
    let RED =         rgb_to_vec3(188u, 55u,  84u);  // rgb(188, 55, 84);
    let ORANGE =      rgb_to_vec3(249u, 142u, 9u);   // rgb(249, 142, 9);
    let YELLOW =      rgb_to_vec3(252u, 255u, 164u); // rgb(252, 255, 164);
    let OTHERYELLOW = rgb_to_vec3(252u, 253u, 228u); // rgb(255, 255, 228)
    let ANOTHERYELLOW = rgb_to_vec3(250u, 228u, 149u); // rgb(250, 228, 149)

    let k = 40.0;
    var dmin = 10000000000.0;

    let pix = 1.0;

    var p = floor(in.position.xy / pix) * pix;

    for (var i = 0; i < 32; i += 1)
    {
        var x = rand(f32(i)) * 2500.0; // uniform_data.resolution.x;
        var y = rand(f32(i + 747457)) * 1200.0; // uniform_data.resolution.y;

        let period = (rand(f32(i) * 90275) * 5.0) + 2.0;

        var dx = 0.0; // cos(uniform_data.time / period) * 200.0;
        var dy = sin(uniform_data.time / period) * 200.0;

        if i == 0 {
            x = uniform_data.mouse_pos.x;
            y = uniform_data.mouse_pos.y;
            dx = 0.0;
            dy = 0.0;
        }

        let r = rand(f32(i + 97212041)) * 90 + 90;
        let d = length(p - vec2<f32>(x + dx, y + dy)) - r;
        dmin = smin(dmin, d, k);
    }

    if dmin < -50.0
    {
        return vec4<f32>(PURPLE, 1.0);
    }
    else if dmin < -20.0
    {
        return vec4<f32>(RED, 1.0);
    }
    else if 0.0 < dmin && dmin < 20.0
    {
        let t = smoothstep(0.0, 20.0, dmin); // goes from 0 to 1

        let color_a = vec3<f32>(0.6, 0.2, 0.75);
        let color_b = vec3<f32>(0.2, 0.5, 0.3);

        let color = mix(color_a, color_b, t);

        return vec4<f32>(ORANGE, 1.0);
    }
    else if 50.0 < dmin && dmin < 60.0
    {
        return vec4<f32>(YELLOW, 1.0);
    }
    else if 70.0 < dmin
    {
        let h = fract(p.y / 200.0);

        if (h < 0.5)
        {
            return vec4<f32>(OTHERYELLOW, 1.0);
        }
        else
        {
            return vec4<f32>(ANOTHERYELLOW, 1.0);
        }
    }

    let bg_color = vec3<f32>(0.0, 0.0, 0.0);

    return vec4<f32>(bg_color, 1.0);
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    // var c = vec4<f32>(0.3, 0.3, 0.3, 0.5);


    // c.x = pow(c.x, 2.0);
    // c.y = pow(c.y, 2.0);
    // c.z = pow(c.z, 2.0);


    // var c = mix(in.color, get_lava_lamp_color(in), 1.0);;
    // return c;

    return get_lava_lamp_color(in);
}
