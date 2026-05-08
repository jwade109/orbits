use crate::prelude::*;
use image::RgbaImage;
use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy)]
struct AsteroidShapeParameter {
    amplitude: f32,
    frequency: u32,
    phase: f32,
}

#[derive(Debug, Clone)]
pub struct Asteroid {
    seed: u64,
    base_radius: f32,
    parameters: Vec<AsteroidShapeParameter>,
    deposits: Vec<(Vec2, f32, u8)>,
    craters: Vec<(Vec2, f32)>,
    noise: Perlin,
    deleted_zones: HashSet<Zone>,
    changes: HashSet<Zone>,
}

impl Asteroid {
    pub fn random(base_radius: f32, seed: Option<u64>) -> Self {
        let seed = seed.unwrap_or(randint(1000, 1000000) as u64);
        let mut rng = StdRng::seed_from_u64(seed);

        let noise = Perlin::new(rng.random());
        let n_params = rng.random_range(5..=8);
        let mut frequency = 1;
        let parameters = (0..n_params)
            .into_iter()
            .map(|_| {
                let amplitude = rng.random_range(0.0..=0.5) / frequency as f32;
                let phase = rng.random_range(0.0..=2.0 * PI);
                let param: AsteroidShapeParameter = AsteroidShapeParameter {
                    amplitude,
                    frequency,
                    phase,
                };
                frequency += rng.random_range(2..=5);
                param
            })
            .collect();

        let n_deposits = rng.random_range(8..30);

        let mut s = Self {
            seed,
            base_radius,
            parameters,
            deposits: Vec::new(),
            noise,
            craters: Vec::new(),
            deleted_zones: HashSet::new(),
            changes: HashSet::new(),
        };

        let max_r = s.max_radius();
        let n_craters = rng.random_range(4..28);

        for _ in 0..n_craters {
            let x = rng.random_range(-max_r..=max_r);
            let y = rng.random_range(-max_r..=max_r);
            let r = rng.random_range(10.0..max_r / 3.0);
            s.craters.push((Vec2::new(x, y), r))
        }

        let deposits = (0..n_deposits)
            .map(|_| {
                let theta = rng.random_range(0.0..=2.0 * PI);
                let r_max = s.radius_at(theta);
                let r = rng.random_range(2.0..=r_max - 1.0);
                let size = rng.random_range(3.0..=base_radius * 0.3);
                let p = rotate(Vec2::X * r, theta);
                (p, size, rng.random_range(120..240))
            })
            .collect();

        s.deposits = deposits;

        s
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn face_dir(&self, p: Vec2) -> Vec3 {
        let theta = p.to_angle();
        let r = self.base_radius;
        let d = p.length();
        let angle = d / r * 0.5 * PI;
        let xy = rotate(Vec2::X, theta) * angle.sin();
        let z = angle.cos();
        xy.extend(z).normalize_or_zero()
    }

    pub fn base_radius(&self) -> f32 {
        self.base_radius
    }

    pub fn radius_at(&self, theta: f32) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r += param.amplitude * (param.frequency as f32 * theta + param.phase).cos()
        }
        r * self.base_radius
    }

    pub fn max_radius(&self) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r += param.amplitude;
        }
        r * self.base_radius
    }

    pub fn min_radius(&self) -> f32 {
        let mut r = 1.0;
        for param in &self.parameters {
            r -= param.amplitude;
        }
        r * self.base_radius
    }

    pub fn contains(&self, p: Vec2) -> bool {
        self.signed_distance(p) >= 0.0 && self.deleted_zones().all(|z| !z.aabb().contains(p))
    }

    pub fn signed_distance(&self, p: Vec2) -> f32 {
        let theta = p.to_angle();
        let r = p.length();
        let r_ast = self.radius_at(theta);
        r_ast - r
    }

    pub fn get_deposit(&self, p: Vec2) -> Option<u8> {
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

    pub fn noise_a(&self, p: Vec2) -> f32 {
        let scale_1 = 1000.0;
        self.noise
            .get([p.x as f64 / scale_1, p.y as f64 / scale_1, 0.0]) as f32
    }

    pub fn noise_b(&self, p: Vec2) -> f32 {
        let scale_2 = 30.0;
        self.noise
            .get([p.x as f64 / scale_2, p.y as f64 / scale_2, 0.0]) as f32
    }

    pub fn noise_c(&self, p: Vec2) -> f32 {
        self.noise.get([p.x as f64 / 60.0, p.y as f64 / 60.0, 0.0]) as f32
    }

    pub fn noise(&self, p: Vec2) -> f32 {
        let n1 = self.noise_a(p);
        let n2 = self.noise_b(p);
        n1 + n2
    }

    pub fn sample_color(&self, p: Vec2, highlight_deposits: bool) -> Option<[u8; 4]> {
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

    pub fn sprite_name(&self, zone: Zone) -> String {
        format!(
            "ast-{}-{:0.4}-{}-{}",
            self.seed, self.base_radius, zone.index, zone.size
        )
    }

    pub fn delete_terrain(&mut self, p: DVec2) {
        let z = get_zone(p, 10);
        let before = self.deleted_zones.len();
        if self.contains(z.aabb().center) {
            self.deleted_zones.insert(z);
        }
        let after = self.deleted_zones.len();
        if before != after {
            let z = get_zone(p, 100);
            if self.contains_zone(z) {
                self.changes.insert(z);
            }
        }
    }

    pub fn deleted_zones(&self) -> impl Iterator<Item = &Zone> + use<'_> {
        self.deleted_zones.iter()
    }

    pub fn contains_zone(&self, z: Zone) -> bool {
        self.zones().find(|o| *o == z).is_some()
    }

    pub fn zones(&self) -> impl Iterator<Item = Zone> + use<'_> {
        let size = 100;
        let max_r = self.max_radius();
        let n = (max_r as f64 / size as f64).ceil() as i32;
        (-n..=n).flat_map(move |x| {
            (-n..=n).map(move |y| Zone {
                size: size,
                index: IVec2 { x, y },
            })
        })
    }

    pub fn is_changed(&self, z: Zone) -> bool {
        self.changes.contains(&z)
    }

    pub fn clear_changed(&mut self, z: Zone) {
        self.changes.remove(&z);
    }

    pub fn changed_zones(&self) -> impl Iterator<Item = &Zone> + use<'_> {
        self.changes.iter()
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Zone {
    size: u32,
    index: IVec2,
}

impl Zone {
    pub fn aabb(&self) -> AABB {
        let lower = self.size as f64 * self.index.as_dvec2();
        let upper = self.size as f64 * (self.index + IVec2::ONE).as_dvec2();
        AABB::from_arbitrary(aabb_stopgap_cast(lower), aabb_stopgap_cast(upper))
    }
}

pub fn get_zone(pos: DVec2, size: u32) -> Zone {
    let index = vfloor_f64(pos / size as f64);
    Zone { size, index }
}

fn write_pixel(img: &mut RgbaImage, p: IVec2, color: [u8; 4]) {
    if p.x < 0 || p.y < 0 || p.x >= img.width() as i32 || p.y >= img.height() as i32 {
        return;
    }
    if let Some(pixel) =
        img.get_pixel_mut_checked(p.x as u32, (img.height() as i32 - p.y - 1) as u32)
    {
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

    let mut img = RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));

    let world_to_img = |p: Vec2| -> IVec2 {
        let u = viewport.to_normalized(p);
        vfloor(u * Vec2::new(width as f32, height as f32))
    };

    for w in 0..width {
        let sx = (w as f32 + 0.5) as f32 / width as f32;
        for h in 0..height {
            let sy = (h as f32 + 0.5) as f32 / height as f32;
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
