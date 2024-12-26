use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;

pub trait Propagate {
    fn pv(&self) -> PV;

    fn relative_to(&self) -> Option<ObjectId>;

    fn step(&mut self, epoch: Duration, forcing: Vec2, state: &OrbitalSystem);
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2, Option<ObjectId>),
    Kepler(KeplerPropagator),
}

impl Propagate for Propagator {
    fn step(&mut self, dt: Duration, forcing: Vec2, state: &OrbitalSystem) {
        match self {
            Propagator::Kepler(k) => k.step(dt, forcing),
            Propagator::Fixed(_, _) => (),
        };
    }

    fn relative_to(&self) -> Option<ObjectId> {
        match self {
            Propagator::Kepler(k) => Some(k.primary),
            Propagator::Fixed(_, o) => *o,
        }
    }

    fn pv(&self) -> PV {
        match self {
            Propagator::Kepler(k) => k.orbit.pv(),
            Propagator::Fixed(p, _) => PV::new(*p, Vec2::ZERO),
        }
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

impl From<Vec2> for Propagator {
    fn from(x: Vec2) -> Propagator {
        Propagator::Fixed(x, None)
    }
}
