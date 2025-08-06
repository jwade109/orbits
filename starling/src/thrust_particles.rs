// use crate::canvas::Canvas;
// use crate::scenes::CameraProjection;
// use bevy::prelude::{Alpha, Mix, Srgba};
use crate::prelude::*;

#[derive(Debug)]
pub struct ThrustParticle {
    pub pv: PV,
    pub age: Nanotime,
    pub lifetime: Nanotime,
    pub initial_color: [f32; 4],
    pub final_color: [f32; 4],
    pub depth: f32,
    pub angle: f32,
}

impl ThrustParticle {
    fn new(pv: PV, angle: f32, initial_color: [f32; 4], final_color: [f32; 4]) -> Self {
        Self {
            pv,
            age: Nanotime::zero(),
            initial_color,
            final_color,
            lifetime: Nanotime::secs_f32(rand(1.2, 3.0)),
            depth: rand(0.0, 1000.0),
            angle,
        }
    }

    fn step(&mut self) {
        self.pv.pos += self.pv.vel * PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
        self.pv.vel *= 0.97;

        // if self.pv.pos.y < 0.0 && self.pv.vel.y < 0.0 {
        //     let vx = self.pv.vel.x;
        //     let mag = self.pv.vel.y.abs() * rand(0.6, 0.95) as f64;
        //     let angle = rand(0.0, 0.25);
        //     self.pv.vel = rotate_f64(DVec2::X * mag, angle as f64);
        //     if rand(0.0, 1.0) < 0.5 {
        //         self.pv.vel.x *= -1.0;
        //     }
        //     self.pv.vel.x += vx;
        // }
        self.age += PHYSICS_CONSTANT_DELTA_TIME;
    }
}

#[derive(Debug)]
pub struct ThrustParticleEffects {
    pub particles: Vec<ThrustParticle>,
}

fn mix(c1: [f32; 4], c2: [f32; 4], s: f32) -> [f32; 4] {
    let mut ret = [0.0; 4];
    for i in 0..4 {
        ret[i] = c1[i] * (1.0 - s) + c2[i] * s;
    }
    ret
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
        self.particles
            .retain(|p: &ThrustParticle| p.age < p.lifetime || rand(0.0, 1.0) < 0.2);
    }

    pub fn add(&mut self, body: &RigidBody, part: &InstantiatedPart) {
        if let Some((t, d)) = part.as_thruster() {
            if !part.is_built() {
                return;
            }
            for _ in 0..2 {
                let pos = rotate_f64(part.center_meters().as_dvec2(), body.angle);
                let ve = t.exhaust_velocity as f64 / 20.0 + 30.0 * d.throttle() as f64;
                let u = rotate_f64(rotate_f64(DVec2::X, part.rotation().to_angle()), body.angle);
                let vel = randvec(2.0, 4.0).as_dvec2() + u * -ve * rand(0.6, 1.0) as f64;
                let pv = body.pv + PV::from_f64(pos, vel);
                let initial_color = mix(t.primary_color, t.secondary_color, rand(0.1, 0.7));
                // let final_color = WHITE.mix(&DARK_GRAY, rand(0.3, 0.9)).with_alpha(0.4);
                self.particles.push(ThrustParticle::new(
                    pv,
                    (body.angle + part.rotation().to_angle()) as f32,
                    initial_color,
                    [1.0, 1.0, 1.0, 0.7],
                ));
            }
        }
    }
}

pub fn add_particles_from_vehicle(
    particles: &mut ThrustParticleEffects,
    vehicle: &Vehicle,
    body: &RigidBody,
) {
    for (_, part) in vehicle.parts() {
        if let Some((t, d)) = part.as_thruster() {
            if !d.is_thrusting(t) || t.is_rcs() {
                continue;
            }

            particles.add(body, part);
        }
    }
}
