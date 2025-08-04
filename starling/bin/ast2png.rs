use clap::Parser;
use image::RgbaImage;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use starling::prelude::*;

/// Converts ship file to PNG
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Asteroid seed
    #[arg(long, short)]
    seed: Option<u64>,

    /// Render image with this width
    #[arg(long, short)]
    width: u32,

    /// Resize resultant image to this size
    #[arg(long, default_value = "400")]
    resize_to: u32,

    /// Resize resultant image to this size
    #[arg(long, short, default_value = "400")]
    radius: f32,

    /// Apply light cast at this angle
    #[arg(long, short)]
    light_angle: Option<f32>,

    /// Draw debug info
    #[arg(long, short)]
    debug: bool,

    /// Highlight ore deposits
    #[arg(long, short)]
    highlight_deposits: bool,

    /// X chunk coordinate
    #[arg(short, allow_hyphen_values = true)]
    x: Option<i32>,

    /// Y chunk coordinate
    #[arg(short, allow_hyphen_values = true)]
    y: Option<i32>,

    /// Focus on particular point on the surface
    #[arg(short('a'), allow_hyphen_values = true)]
    surface_angle: Option<f32>,
}

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
    craters: Vec<(Vec2, f32)>,
    noise: Perlin,
}

impl Asteroid {
    fn random(base_radius: f32, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let noise = Perlin::new(rng.gen());
        let n_params = rng.gen_range(5..=8);
        let mut frequency = 1;
        let parameters = (0..n_params)
            .into_iter()
            .map(|_| {
                let amplitude = rng.gen_range(0.0..=0.5) / frequency as f32;
                let phase = rng.gen_range(0.0..=2.0 * PI);
                let param: AsteroidShapeParameter = AsteroidShapeParameter {
                    amplitude,
                    frequency,
                    phase,
                };
                frequency += rng.gen_range(2..=5);
                param
            })
            .collect();

        let n_deposits = rng.gen_range(8..30);

        let mut s = Self {
            base_radius,
            parameters,
            deposits: Vec::new(),
            noise,
            craters: Vec::new(),
        };

        let max_r = s.max_radius();
        let n_craters = rng.gen_range(4..28);

        for _ in 0..n_craters {
            let x = rng.gen_range(-max_r..=max_r);
            let y = rng.gen_range(-max_r..=max_r);
            let r = rng.gen_range(10.0..max_r / 3.0);
            s.craters.push((Vec2::new(x, y), r))
        }

        let deposits = (0..n_deposits)
            .map(|_| {
                let theta = rng.gen_range(0.0..=2.0 * PI);
                let r_max = s.radius_at(theta);
                let r = rng.gen_range(2.0..=r_max - 1.0);
                let size = rng.gen_range(3.0..=base_radius * 0.3);
                let p = rotate(Vec2::X * r, theta);
                (p, size, rng.gen_range(120..240))
            })
            .collect();

        s.deposits = deposits;

        s
    }

    fn face_dir(&self, p: Vec2) -> Vec3 {
        let theta = p.to_angle();
        let r = self.base_radius;
        let d = p.length();
        let angle = d / r * 0.5 * PI;
        let xy = rotate(Vec2::X, theta) * angle.sin();
        let z = angle.cos();
        xy.extend(z).normalize_or_zero()
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

    fn min_radius(&self) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r -= param.amplitude;
        }
        r * self.base_radius
    }

    fn contains(&self, p: Vec2) -> bool {
        self.signed_distance(p) >= 0.0
    }

    fn signed_distance(&self, p: Vec2) -> f32 {
        let theta = p.to_angle();
        let r = p.length();
        let r_ast = self.radius_at(theta);
        r_ast - r
    }

    fn get_deposit(&self, p: Vec2) -> Option<u8> {
        if self.noise_c(p) > 0.0 {
            return None;
        }
        self.deposits.iter().find_map(|(c, r, value)| {
            let p = p - c;
            (p.length() < *r).then(|| *value)
            // let k = Vec3::new(-0.866025404, 0.5, 0.577350269);
            // let kxy = Vec2::new(k.x, k.y);
            // let mut p = p.abs();
            // p -= 2.0 * kxy.dot(p).min(0.0) * kxy;
            // p -= Vec2::new(p.x.clamp(-k.z * r, k.z * r), *r);
            // let sd = p.length() * p.y.signum();
            // (sd < 0.0).then(|| *value)
        })
    }

    fn noise_a(&self, p: Vec2) -> f32 {
        let scale_1 = 1000.0;
        self.noise
            .get([p.x as f64 / scale_1, p.y as f64 / scale_1, 0.0]) as f32
    }

    fn noise_b(&self, p: Vec2) -> f32 {
        let scale_2 = 30.0;
        self.noise
            .get([p.x as f64 / scale_2, p.y as f64 / scale_2, 0.0]) as f32
    }

    fn noise_c(&self, p: Vec2) -> f32 {
        self.noise.get([p.x as f64 / 60.0, p.y as f64 / 60.0, 0.0]) as f32
    }

    fn noise(&self, p: Vec2) -> f32 {
        let n1 = self.noise_a(p);
        let n2 = self.noise_b(p);
        // let n3 = self.noise_c(p);
        // let n4 = self.noise.get([p.x as f64 * 10.0, p.y as f64 * 10.0, 0.0]) as f32;
        n1 + n2
    }

    fn sample_noise_color(&self, p: Vec2) -> Option<[u8; 4]> {
        if !self.contains(p) {
            return None;
        }

        let n_to_u8 = |n: f32| ((n.clamp(-1.0, 1.0) / 2.0 + 0.5) * 255.0).round() as u8;

        Some([
            n_to_u8(self.noise_a(p)),
            n_to_u8(self.noise_b(p)),
            n_to_u8(self.noise_c(p)),
            255,
        ])
    }

    fn sample_color(&self, p: Vec2, highlight_deposits: bool) -> Option<[u8; 4]> {
        if !self.contains(p) {
            return None;
        }
        let n = self.noise(p);

        let mut c = if n < 0.0 {
            if let Some(v) = self.get_deposit(p) {
                if highlight_deposits {
                    [255, 100, 0, 255]
                } else {
                    [v, v, v, 255]
                }
            } else {
                [105, 105, 105, 255]
            }
        } else if n < 0.8 {
            [110, 110, 110, 255]
        } else {
            [115, 115, 115, 255]
        };

        let crater_delta = (n > 0.2)
            .then(|| {
                self.craters.iter().find_map(|(c, r)| {
                    let d = c.distance(p);
                    ((d - r).abs() < 4.0).then(|| [10, 10, 10])
                })
            })
            .flatten();

        if let Some(delta) = crater_delta {
            for i in 0..3 {
                if c[i] as i32 + delta[i] as i32 <= 255 {
                    c[i] += delta[i];
                } else {
                    c[i] = 255;
                }
            }
        }

        Some(c)
    }
}

fn write_pixel(img: &mut RgbaImage, p: IVec2, color: [u8; 4]) {
    if p.x < 0 || p.y < 0 || p.x >= img.width() as i32 || p.y >= img.height() as i32 {
        return;
    }
    if let Some(pixel) = img.get_pixel_mut_checked(p.x as u32, p.y as u32) {
        pixel.0 = color;
    }
}

fn attenuate_light(mut color: [u8; 4], dot: f32) -> [u8; 4] {
    let dot = dot.max(0.04);
    for i in 0..3 {
        // if dot < 0.8 {
        let new_color = (color[i] as f32 * dot).clamp(0.0, 255.0);
        color[i] = new_color as u8;
        // }
    }
    color
}

pub fn make_asteroid_image(
    ast: &Asteroid,
    viewport: AABB,
    width: u32,
    light_dir: Option<f32>,
    debug_info: bool,
    highlight_deposits: bool,
) -> RgbaImage {
    let max_r = ast.max_radius();
    let min_r = ast.min_radius();
    let height = (width as f32 * viewport.span.y / viewport.span.x) as u32;

    let mut img = RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]));

    let world_to_img = |p: Vec2| -> IVec2 {
        let u = viewport.to_normalized(p);
        vround(u * Vec2::new(width as f32, height as f32))
    };

    for w in 0..width {
        let sx = w as f32 / width as f32;
        for h in 0..height {
            let sy = h as f32 / height as f32;
            let p = viewport.from_normalized(Vec2::new(sx, sy));
            let color = match ast.sample_color(p, highlight_deposits) {
                Some(c) => c,
                None => continue,
            };

            let color = if let Some(a) = light_dir {
                let light_dir = rotate(Vec2::X, a).normalize_or_zero().extend(0.0);
                let face_dir = ast.face_dir(p);
                let dot = light_dir.dot(-face_dir);
                attenuate_light(color, dot)
            } else {
                color
            };

            write_pixel(&mut img, UVec2::new(w, h).as_ivec2(), color);
        }
    }

    if debug_info {
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

        for theta in linspace(0.0, 2.0 * PI, 100) {
            let p = rotate(Vec2::X * max_r, theta);
            let q = world_to_img(p);
            write_pixel(&mut img, q, [255, 50, 50, 255]);

            let p = rotate(Vec2::X * min_r, theta);
            let q = world_to_img(p);
            write_pixel(&mut img, q, [255, 50, 50, 255]);

            let r = ast.radius_at(theta);
            let p = rotate(Vec2::X * r, theta);
            let q = world_to_img(p);
            write_pixel(&mut img, q, [50, 255, 255, 255]);
        }
    }

    img
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    dbg!(&args);

    let seed = args.seed.unwrap_or(randint(1000, 1000000) as u64);

    println!("Seed: {}", seed);

    let ast = Asteroid::random(args.radius, seed);

    let viewport = if let Some((x, y)) = args.x.zip(args.y) {
        let center = Vec2::new(x as f32 * 100.0, y as f32 * 100.0);
        let span = Vec2::splat(100.0);
        AABB::new(center, span)
    } else if let Some(a) = args.surface_angle {
        let radius = ast.radius_at(a);
        let p = rotate(Vec2::X * radius, a);
        AABB::new(p, Vec2::splat(100.0))
    } else {
        AABB::new(Vec2::ZERO, Vec2::splat(ast.max_radius() * 2.0))
    };

    let img = make_asteroid_image(
        &ast,
        viewport,
        args.width,
        args.light_angle,
        args.debug,
        args.highlight_deposits,
    );

    let img = image::DynamicImage::from(img);
    let img = img.resize(
        args.resize_to,
        args.resize_to,
        image::imageops::FilterType::Nearest,
    );

    let filename = format!("/tmp/asteroid.png");
    Ok(img.save(filename)?)
}
