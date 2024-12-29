use crate::core::*;
use bevy::math::Vec2;
use std::time::Duration;
use crate::orbit::*;

pub trait Propagate {
    fn relative_to(&self) -> Option<ObjectId>;

    fn pv_at(&self, stamp: Duration) -> Option<PV>;
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2, Option<ObjectId>),
    Kepler(KeplerPropagator),
}

impl Propagate for Propagator {
    fn pv_at(&self, stamp: Duration) -> Option<PV> {
        match self {
            Propagator::Kepler(k) => Some(k.orbit.pv_at_time(stamp)),
            Propagator::Fixed(p, _) => Some(PV::new(*p, Vec2::ZERO)),
        }
    }

    fn relative_to(&self) -> Option<ObjectId> {
        match self {
            Propagator::Kepler(k) => Some(k.primary),
            Propagator::Fixed(_, o) => *o,
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
