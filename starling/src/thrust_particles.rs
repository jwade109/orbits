// use crate::canvas::Canvas;
// use crate::scenes::CameraProjection;
// use bevy::prelude::{Alpha, Mix, Srgba};
use crate::prelude::*;

#[derive(Debug)]
pub struct ThrustParticle {
    pub parent: EntityId,
    pub pv: PV,
    pub atmo: f32,
    pub age: Nanotime,
    pub lifetime: Nanotime,
    pub initial_color: [f32; 4],
    pub final_color: [f32; 4],
    pub depth: f32,
    pub angle: f32,
    pub scale: f32,
}

impl ThrustParticle {
    fn new(
        parent: EntityId,
        pv: PV,
        atmo: f32,
        angle: f32,
        initial_color: [f32; 4],
        final_color: [f32; 4],
        scale: f32,
    ) -> Self {
        Self {
            parent,
            pv: pv,
            atmo: atmo.clamp(0.0, 1.0),
            age: Nanotime::zero(),
            initial_color,
            final_color,
            lifetime: Nanotime::secs_f32(rand(1.2, 2.0) * (0.1 + atmo * 0.9)),
            depth: rand(0.0, 1000.0),
            angle,
            scale,
        }
    }

    fn step(&mut self) {
        self.pv.pos += self.pv.vel * PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
        self.pv.vel *= 1.0 - 0.04 * self.atmo as f64;
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

    pub fn add(&mut self, parent: EntityId, body: &RigidBody, part: &InstantiatedPart, atmo: f32) {
        if let Some((t, d)) = part.as_thruster() {
            if !part.is_built() {
                return;
            }

            let atmo = if t.is_rcs { 0.0 } else { atmo };

            let n = 2 + ((1.0 - atmo) * 8.0).round() as u32;

            let pos = rotate_f64(part.center_meters().as_dvec2(), body.angle);

            for _ in 0..n {
                let ve = t.exhaust_velocity as f64 / 20.0 + 30.0 * d.throttle() as f64;
                let u = rotate_f64(rotate_f64(DVec2::X, part.rotation().to_angle()), body.angle);
                let vel = randvec(2.0, 4.0).as_dvec2() + u * -ve * rand(0.6, 1.0) as f64;
                let spread_angle = (1.0 - atmo) * rand(-0.5, 0.5);
                let vel = rotate_f64(vel, spread_angle as f64) * t.particle_scale as f64;
                let pv = body.pv + PV::from_f64(pos, vel);
                let initial_color = mix(t.primary_color, t.secondary_color, rand(0.1, 0.7));
                self.particles.push(ThrustParticle::new(
                    parent,
                    pv,
                    atmo,
                    (body.angle + part.rotation().to_angle()) as f32 + spread_angle,
                    initial_color,
                    [1.0, 1.0, 1.0, 0.7],
                    t.particle_scale,
                ));
            }
        }
    }
}

pub fn add_particles_from_vehicle(
    particles: &mut ThrustParticleEffects,
    parent: EntityId,
    vehicle: &Vehicle,
    body: &RigidBody,
    atmo: f32,
) {
    for (_, part) in vehicle.parts() {
        if let Some((t, d)) = part.as_thruster() {
            if !d.is_thrusting(t) {
                continue;
            }

            particles.add(parent, body, part, atmo);
        }
    }
}
