use bary_core::prelude::*;
use image::{ColorType, DynamicImage};

pub struct Particle {
    pos: Vec2,
    vel: Vec2,
    remaining_time: f32,
    total_time: f32,
    color: BColor,
}

fn random_color() -> BColor {
    if chance(0.3) {
        BColor::YELLOW
    } else {
        BColor::RED
    }
}

impl Particle {
    fn new(pos: Vec2, angle: f32, vscale: f32) -> Self {
        let max_da = (1.0 - vscale / 2.0).clamp(0.0, 1.0);
        let da = rand(-0.4, 0.4) * max_da;
        let a = angle + da;
        let v = rand(130.0, 400.0) * da.cos() * vscale;
        let vel = rotate(Vec2::X * v, a);
        let t = rand(0.3, 0.5);

        Self {
            pos,
            vel,
            remaining_time: t,
            total_time: t,
            color: random_color(),
        }
    }

    fn step(&mut self) {
        self.pos += self.vel * DT;
        self.remaining_time -= DT;
        self.vel *= 0.985;
    }

    fn alpha(&self) -> f32 {
        self.remaining_time / self.total_time
    }

    fn color(&self) -> BColor {
        // let a = self.alpha();
        // self.color.mix(&GRAY_800, 1.0 - a)
        self.color
    }
}

const DT: f32 = 0.002;

const WIDTH: u32 = 150;
const HEIGHT: u32 = 150;

pub struct ParticleSim {
    emitter_top: Vec2,
    emitter_bottom: Vec2,
    particles: Vec<Particle>,
    angle: f32,
    angular_velocity: f32,
    velocity_scale: f32,
    emit: bool,
    switch_dur: f32,
}

impl ParticleSim {
    fn new() -> Self {
        Self {
            emitter_top: Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            emitter_bottom: Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            particles: Vec::new(),
            angle: 0.0,
            angular_velocity: 0.0,
            velocity_scale: rand(0.1, 2.0),
            emit: true,
            switch_dur: 3.0,
        }
    }

    fn step(&mut self) {
        self.angular_velocity += rand(-DT, DT);
        self.angular_velocity = self.angular_velocity.clamp(-1.0, 1.0);
        self.angle += DT * self.angular_velocity;

        self.velocity_scale += rand(-DT, DT) * 6.0;
        self.velocity_scale = self.velocity_scale.clamp(0.1, 5.0);

        self.switch_dur -= DT;

        self.particles.iter_mut().for_each(|p| p.step());

        if self.emit {
            for _ in 0..50 {
                let s = rand(0.0, 1.0);
                let e = self.emitter_bottom.lerp(self.emitter_top, s);
                let e = e + randvec(1.0, 2.0);
                let p = Particle::new(e, self.angle, self.velocity_scale);
                self.particles.push(p);
            }
        }

        if self.switch_dur < 0.0 {
            self.emit = !self.emit;
            if self.emit {
                self.switch_dur = rand(3.0, 5.0);
            } else {
                self.switch_dur = rand(0.4, 1.0);
            }
        }

        self.particles.retain(|p| p.remaining_time > 0.0);
    }
}

fn render_particles(sim: &ParticleSim) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    if sim.particles.is_empty() {
        return Ok(DynamicImage::new(WIDTH, HEIGHT, ColorType::Rgba8));
    }

    let mut img = DynamicImage::new(WIDTH, HEIGHT, ColorType::Rgba8);

    let editable = img.as_mut_rgba8().ok_or("Expected a buffer")?;

    for particle in &sim.particles {
        let p = particle.pos.as_ivec2();

        if p.x < 0 || p.y < 0 {
            continue;
        }

        let p = p.as_uvec2();

        let Some(pixel) = editable.get_pixel_mut_checked(p.x, p.y) else {
            continue;
        };

        pixel.0 = particle.color.to_u8();
    }

    Ok(img)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut sim = ParticleSim::new();

    loop {
        for _ in 0..10 {
            sim.step();
        }

        let img = render_particles(&sim)?;

        img.save("/tmp/out.png")?;

        std::thread::sleep(std::time::Duration::from_millis(40));
    }
}
