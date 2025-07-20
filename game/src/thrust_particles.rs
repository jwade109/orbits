use crate::canvas::Canvas;
use crate::scenes::CameraProjection;
use bevy::prelude::{Alpha, Mix, Srgba};
use starling::prelude::*;

#[derive(Debug)]
struct ThrustParticle {
    pv: PV,
    age: Nanotime,
    lifetime: Nanotime,
    initial_color: Srgba,
    final_color: Srgba,
    depth: f32,
}

impl ThrustParticle {
    fn new(pv: PV, initial_color: Srgba, final_color: Srgba) -> Self {
        Self {
            pv,
            age: Nanotime::zero(),
            initial_color,
            final_color,
            lifetime: Nanotime::secs_f32(rand(1.2, 1.8)),
            depth: rand(0.0, 1000.0),
        }
    }

    fn step(&mut self) {
        self.pv.pos += self.pv.vel * PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
        self.pv.vel *= 0.92;

        if self.pv.pos.y < 0.0 && self.pv.vel.y < 0.0 {
            let vx = self.pv.vel.x;
            let mag = self.pv.vel.y.abs() * rand(0.6, 0.95) as f64;
            let angle = rand(0.0, 0.25);
            self.pv.vel = rotate_f64(DVec2::X * mag, angle as f64);
            if rand(0.0, 1.0) < 0.5 {
                self.pv.vel.x *= -1.0;
            }
            self.pv.vel.x += vx;
        }
        self.age += PHYSICS_CONSTANT_DELTA_TIME;
    }
}

#[derive(Debug)]
pub struct ThrustParticleEffects {
    particles: Vec<ThrustParticle>,
}

impl ThrustParticleEffects {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
        }
    }

    pub fn step(&mut self) {
        for part in &mut self.particles {
            part.step();
        }
        self.particles.retain(|p| p.age < p.lifetime);
    }

    pub fn add(&mut self, _v: &Vehicle, _part: &InstantiatedPart) {
        // if let Some((t, d)) = part.as_thruster() {
        //     let pos = rotate(part.center_meters(), v.angle());
        //     let ve = t.exhaust_velocity / 16.0 + 30.0 * d.throttle();
        //     let u = rotate(rotate(Vec2::X, part.rotation().to_angle()), v.angle());
        //     let vel = randvec(2.0, 10.0) + u * -ve * rand(0.6, 1.0);
        //     let pv = v.pv + PV::from_f64(pos, vel);
        //     let c1 = to_srgba(t.primary_color);
        //     let c2 = to_srgba(t.secondary_color);
        //     let initial_color = c1.mix(&c2, rand(0.0, 1.0));
        //     let final_color = WHITE.mix(&DARK_GRAY, rand(0.3, 0.9)).with_alpha(0.4);
        //     self.particles
        //         .push(ThrustParticle::new(pv, initial_color, final_color));
        // }
    }

    pub fn draw(&self, canvas: &mut Canvas, ctx: &impl CameraProjection) {
        for particle in &self.particles {
            let p = ctx.w2c(particle.pv.pos_f32());
            let age = particle.age.to_secs();
            let alpha = (1.0 - age / particle.lifetime.to_secs())
                .powi(3)
                .clamp(0.0, 1.0);
            let color = particle
                .initial_color
                .mix(&particle.final_color, age.clamp(0.0, 1.0).sqrt());
            let size = 1.0 + age * 12.0;
            let ramp_up = (age * 40.0).clamp(0.0, 1.0);
            let stretch = (8.0 * (1.0 - age * 2.0)).max(1.0);
            let angle = particle.pv.vel.to_angle() as f32;
            canvas
                .sprite(
                    p,
                    angle,
                    "cloud",
                    particle.depth,
                    Vec2::new(size * stretch * ramp_up, size * ramp_up) * ctx.scale(),
                )
                .set_color(color.with_alpha(color.alpha * alpha));
        }
    }
}
