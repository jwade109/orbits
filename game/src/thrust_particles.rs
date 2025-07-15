use crate::canvas::Canvas;
use crate::scenes::surface::to_srbga;
use crate::scenes::CameraProjection;
use bevy::color::palettes::css::*;
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

const MAX_PARTICLE_AGE_SECONDS: Nanotime = Nanotime::secs_f32(3.0);

const NOMINAL_DT: Nanotime = Nanotime::millis(20);

impl ThrustParticle {
    fn new(pv: PV, initial_color: Srgba, final_color: Srgba) -> Self {
        Self {
            pv,
            age: Nanotime::zero(),
            initial_color,
            final_color,
            lifetime: MAX_PARTICLE_AGE_SECONDS * rand(0.5, 1.0),
            depth: rand(0.0, 1000.0),
        }
    }

    fn step(&mut self) {
        self.pv.pos += self.pv.vel * NOMINAL_DT.to_secs_f64();
        self.pv.vel *= 0.96;

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
        self.age += NOMINAL_DT;
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

    pub fn add(&mut self, v: &Vehicle, t: &InstanceRef<&Thruster>) {
        let pos = rotate(t.center_meters(), v.angle());
        let ve = t.variant.model().exhaust_velocity / 20.0;
        let u = rotate(t.thrust_pointing(), v.angle());
        let vel = randvec(2.0, 10.0) + u * -ve * rand(0.6, 1.0);
        let pv = v.pv + PV::from_f64(pos, vel);
        let c1 = to_srbga(t.variant.model().primary_color);
        let c2 = to_srbga(t.variant.model().secondary_color);
        let initial_color = c1.mix(&c2, rand(0.0, 1.0));
        let final_color = WHITE.mix(&DARK_GRAY, rand(0.3, 0.9)).with_alpha(0.4);
        self.particles
            .push(ThrustParticle::new(pv, initial_color, final_color));
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
