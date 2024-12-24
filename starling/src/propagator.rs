use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

pub trait Propagate {
    fn pv(&self) -> PV;

    // fn epoch(&self) -> Duration;

    fn relative_to(&self) -> Option<ObjectId>;

    fn step(&mut self, epoch: Duration, forcing: Vec2, state: &OrbitalSystem);
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2, Option<ObjectId>),
    NBody(NBodyPropagator),
    Kepler(KeplerPropagator),
}

impl Propagate for Propagator {
    fn step(&mut self, dt: Duration, forcing: Vec2, state: &OrbitalSystem) {
        match self {
            Propagator::NBody(nb) => nb.step(&state.bodies(), dt, forcing),
            Propagator::Kepler(k) => k.step(dt, forcing),
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
        }
    }

    pub fn initial(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        NBodyPropagator {
            pos: pos.into(),
            vel: vel.into(),
        }
    }

    pub fn pv(&self) -> PV {
        PV::new(self.pos, self.vel)
    }

    pub fn step(&mut self, bodies: &[(ObjectId, Vec2, Body)], delta: Duration, forcing: Vec2) {
        let steps = 10; // (delta.as_secs_f32() * self.vel.length() / 3.0).ceil() as u32;
        let dt = delta.as_secs_f32() / steps as f32; // TODO

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
                // euler integration
                let a = compute_a_at(self.pos);
                self.vel += a * dt;
                self.pos += self.vel * dt;
            }

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
                let a = compute_a_at(self.pos) + forcing;
                let v_half = self.vel + a * 0.5 * dt as f32;
                self.pos += v_half * dt as f32;
                let a2 = compute_a_at(self.pos) + forcing;
                self.vel = v_half + a2 * 0.5 * dt as f32;
            }
        });
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeplerPropagator {
    pub epoch: Duration,
    pub primary: ObjectId,
    pub orbit: Orbit,
}

impl KeplerPropagator {
    pub fn new(orbit: Orbit, primary: ObjectId) -> Self {
        KeplerPropagator {
            epoch: Duration::default(),
            primary,
            orbit,
        }
    }

    pub fn epoch(&self) -> Duration {
        self.epoch
    }

    pub fn from_pv(pos: Vec2, vel: Vec2, mass: f32, primary: ObjectId) -> Self {
        let orbit = Orbit::from_pv(pos, vel, mass);
        KeplerPropagator {
            epoch: Duration::default(),
            primary,
            orbit,
        }
    }

    pub fn step(&mut self, dt: Duration, forcing: Vec2) {
        if dt == Duration::default() {
            return;
        }

        if forcing != Vec2::ZERO {
            let mut pv = self.orbit.pv();
            pv.vel += forcing * dt.as_secs_f32();
            self.orbit = Orbit::from_pv(pv.pos, pv.vel, self.orbit.primary_mass);
        }

        let n = self.orbit.mean_motion();
        let m = self.orbit.mean_anomaly();
        let m2 = m + dt.as_secs_f32() * n;
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
