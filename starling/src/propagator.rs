use crate::core::*;
use bevy::math::Vec2;
use std::{collections::VecDeque, time::Duration};

pub trait Propagate {
    fn pv(&self) -> PV;

    fn relative_to(&self) -> Option<ObjectId>;

    fn propagate(&mut self, delta: Duration, state: &OrbitalSystem);
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2, Option<ObjectId>),
    NBody(NBodyPropagator),
    Kepler(KeplerPropagator),
}

#[derive(Debug, Clone)]
struct PropagatorBuffer {
    buffer: VecDeque<Propagator>,
}

impl PropagatorBuffer {
    fn pv_at(&self, stamp: Duration) -> Option<PV> {
        todo!()
    }

    fn propagate_to(&mut self, stamp: Duration) {
        todo!()
    }
}

impl Propagate for Propagator {
    fn propagate(&mut self, delta: Duration, state: &OrbitalSystem) {
        match self {
            Propagator::NBody(nb) => nb.propagate(&state.bodies(), delta),
            Propagator::Kepler(k) => k.propagate(delta),
            Propagator::Fixed(_, _) => (),
        };
    }

    fn relative_to(&self) -> Option<ObjectId> {
        match self {
            Propagator::NBody(_) => None,
            Propagator::Kepler(k) => Some(k.primary),
            Propagator::Fixed(_, o) => *o,
        }
    }

    fn pv(&self) -> PV {
        match self {
            Propagator::NBody(nb) => nb.pv(),
            Propagator::Kepler(k) => k.orbit.pv(),
            Propagator::Fixed(p, _) => PV::new(*p, Vec2::ZERO),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NBodyPropagator {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl NBodyPropagator {
    pub fn new(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        NBodyPropagator {
            pos: pos.into(),
            vel: vel.into(),
            ..NBodyPropagator::default()
        }
    }

    pub fn pv(&self) -> PV {
        PV::new(self.pos, self.vel)
    }

    pub fn propagate(&mut self, bodies: &[(ObjectId, Vec2, Body)], delta: Duration) {
        let steps = 10; // (delta.as_secs_f32() * self.vel.length() / 3.0).ceil() as u32;
        let dt = delta.as_secs_f32() / steps as f32;

        let others = bodies
            .iter()
            .filter(|(_, c, _)| *c != self.pos)
            .collect::<Vec<_>>();

        let compute_a_at = |p: Vec2| -> Vec2 {
            others
                .iter()
                .map(|(_, c, b)| -> Vec2 { gravity_accel(*b, *c, p) })
                .sum()
        };

        (0..steps).for_each(|_| {
            #[cfg(any())]
            {
                // velocity verlet integration
                let a = compute_a_at(self.pos);
                self.pos += self.vel * dt + 0.5 * a * dt * dt;
                let a2 = compute_a_at(self.pos);
                self.vel += 0.5 * (a + a2) * dt;
            }

            #[cfg(all())]
            {
                // synchronized leapfrog integration
                let a = compute_a_at(self.pos);
                let v_half = self.vel + a * 0.5 * dt as f32;
                self.pos += v_half * dt as f32;
                let a2 = compute_a_at(self.pos);
                self.vel = v_half + a2 * 0.5 * dt as f32;
            }
        });
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeplerPropagator {
    pub primary: ObjectId,
    pub orbit: Orbit,
}

impl KeplerPropagator {
    pub fn new(orbit: Orbit, primary: ObjectId) -> Self {
        KeplerPropagator { primary, orbit }
    }

    pub fn from_pv(pos: Vec2, vel: Vec2, body: Body, primary: ObjectId) -> Self {
        let orbit = Orbit::from_pv(pos, vel, body);
        KeplerPropagator { primary, orbit }
    }

    pub fn propagate(&mut self, delta: Duration) {
        if delta == Duration::default() {
            return;
        }

        let n = self.orbit.mean_motion();
        let m = self.orbit.mean_anomaly();
        let m2 = m + delta.as_secs_f32() * n;
        self.orbit.true_anomaly = anomaly_m2t(self.orbit.eccentricity, m2).unwrap_or(f32::NAN);
    }
}

impl From<KeplerPropagator> for Propagator {
    fn from(x: KeplerPropagator) -> Propagator {
        Propagator::Kepler(x)
    }
}

impl From<NBodyPropagator> for Propagator {
    fn from(x: NBodyPropagator) -> Propagator {
        Propagator::NBody(x)
    }
}

impl From<Vec2> for Propagator {
    fn from(x: Vec2) -> Propagator {
        Propagator::Fixed(x, None)
    }
}
