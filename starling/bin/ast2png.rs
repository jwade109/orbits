use image::RgbaImage;
use noise::{NoiseFn, Perlin};
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
struct AsteroidShapeParameter {
    amplitude: f32,
    frequency: u32,
    phase: f32,
}

#[derive(Debug, Clone)]
pub struct Asteroid {
    base_radius: f32,
    parameters: Vec<AsteroidShapeParameter>,
    deposits: Vec<(Vec2, f32, u8)>,
}

impl Asteroid {
    fn random() -> Self {
        let base_radius = rand(80.0, 700.0);

        let n_params = randint(4, 8);
        let mut frequency = 1;
        let parameters = (0..n_params)
            .into_iter()
            .map(|_| {
                let amplitude = rand(0.0, 0.5) / frequency as f32;
                let phase = rand(0.0, 2.0 * PI);
                let param = AsteroidShapeParameter {
                    amplitude,
                    frequency,
                    phase,
                };
                frequency += randint(2, 5) as u32;
                param
            })
            .collect();

        let n_deposits = randint(2, 12);

        let mut s = Self {
            base_radius,
            parameters,
            deposits: Vec::new(),
        };

        let deposits = (0..n_deposits)
            .map(|_| {
                let theta = rand(0.0, 2.0 * PI);
                let r_max = s.radius_at(theta);
                let r = rand(2.0, r_max - 1.0);
                let size = rand(3.0, 32.0);
                let p = rotate(Vec2::X * r, theta);
                (p, size, randint(120, 240) as u8)
            })
            .collect();

        s.deposits = deposits;

        s
    }

    fn radius_at(&self, theta: f32) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r += param.amplitude * (param.frequency as f32 * theta + param.phase).cos()
        }
        r * self.base_radius
    }

    fn max_radius(&self) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r += param.amplitude;
        }
        r * self.base_radius
    }

    fn contains(&self, p: Vec2) -> bool {
        self.depth(p) >= 0.0
    }

    fn depth(&self, p: Vec2) -> f32 {
        let theta = p.to_angle();
        let r = p.length();
        let r_ast = self.radius_at(theta);
        r_ast - r
    }

    fn get_deposit(&self, p: Vec2) -> Option<u8> {
        self.deposits.iter().find_map(|(c, r, value)| {
            let p = p - c;
            let k = Vec3::new(-0.866025404, 0.5, 0.577350269);
            let kxy = Vec2::new(k.x, k.y);
            let mut p = p.abs();
            p -= 2.0 * kxy.dot(p).min(0.0) * kxy;
            p -= Vec2::new(p.x.clamp(-k.z * r, k.z * r), *r);
            let sd = p.length() * p.y.signum();
            (sd < 0.0).then(|| *value)
        })
    }
}

fn write_pixel(img: &mut RgbaImage, p: UVec2, color: [u8; 4]) {
    if let Some(pixel) = img.get_pixel_mut_checked(p.x, p.y) {
        pixel.0 = color;
    }
}

pub fn make_asteroid_image(ast: &Asteroid, width: u32) -> Option<RgbaImage> {
    let perlin = Perlin::new(1);

    let max_r = ast.max_radius();
    let meters_per_pixel = 4.0; // max_r / width as f32 * 2.0;
    let height = width;

    let scale_1 = 100.0;
    let scale_2 = 30.0;

    let center_pixel = UVec2::new(width / 2, height / 2);

    dbg!(width);

    let offset = -center_pixel.as_vec2() * meters_per_pixel;

    let mut img = RgbaImage::new(width, height);

    for w in 0..width {
        let x = w as f32 * meters_per_pixel;
        for h in 0..height {
            let y = h as f32 * meters_per_pixel;
            let p = Vec2::new(x, y) + offset;
            let p1 = perlin.get([p.x as f64 / scale_1, p.y as f64 / scale_1, 0.0]);
            let color = if ast.contains(p) {
                let p2 = perlin.get([p.x as f64 / scale_2, p.y as f64 / scale_2, 0.0]);
                if p1 + p2 < 0.0 {
                    if let Some(v) = ast.get_deposit(p) {
                        [v, v, v, 255]
                    } else {
                        [90, 90, 90, 255]
                    }
                } else {
                    [110, 110, 110, 255]
                }
            } else {
                [0; 4]
            };

            // let depth = ast.depth(p) + p1 as f32 * 40.0;
            // let color = if depth > 100.0 { [0, 0, 0, 255] } else { color };

            write_pixel(&mut img, UVec2::new(w, h), color);
        }
    }

    let world_to_img =
        |p: Vec2| -> UVec2 { (center_pixel.as_ivec2() + vround(p / meters_per_pixel)).as_uvec2() };

    // for (p, size, _value) in &ast.deposits {
    //     let color = [0, 255, 0, 255];
    //     let q = world_to_img(*p);
    //     write_pixel(&mut img, q, color);
    //     for a in linspace(0.0, 2.0 * PI, 100) {
    //         let p = p + rotate(Vec2::X * size, a);
    //         if ast.contains(p) {
    //             let q = world_to_img(p);
    //             write_pixel(&mut img, q, color)
    //         }
    //     }
    // }

    for x in (0..(max_r.ceil() as u32)).step_by(50) {
        for x in [-(x as f32), x as f32] {
            let p = Vec2::new(x, 0.0);
            if ast.contains(p) {
                let q = world_to_img(p);
                write_pixel(&mut img, q, [70, 255, 70, 255]);
            }
        }
    }

    for y in (0..(max_r.ceil() as u32)).step_by(50) {
        for y in [-(y as f32), y as f32] {
            let p = Vec2::new(0.0, y);
            if ast.contains(p) {
                let q = world_to_img(p);
                write_pixel(&mut img, q, [70, 255, 70, 255]);
            }
        }
    }

    for theta in linspace(0.0, 2.0 * PI, 50) {
        let p = rotate(Vec2::X * max_r, theta);
        let q = world_to_img(p);
        write_pixel(&mut img, q, [255, 50, 50, 255]);
    }

    write_pixel(&mut img, center_pixel, [50, 50, 255, 255]);

    Some(img)
}

fn main() {
    let ast = Asteroid::random();

    let img = make_asteroid_image(&ast, 300).unwrap();

    let _ = dbg!(img.save("/tmp/asteroid.png"));
}
