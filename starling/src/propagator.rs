use crate::core::*;
use crate::orbit::*;
use bevy::math::Vec2;
use std::{collections::VecDeque, time::Duration};

pub trait Propagate {
    fn pv(&self) -> PV;

    fn epoch(&self) -> Duration;

    fn next(&self, state: &OrbitalSystem) -> Self;

    fn relative_to(&self) -> Option<ObjectId>;

    fn propagate_to(&mut self, epoch: Duration, state: &OrbitalSystem);
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2, Option<ObjectId>),
    NBody(NBodyPropagator),
    Kepler(KeplerPropagator),
}

#[derive(Debug, Clone, Default)]
pub struct PropagatorBuffer(pub VecDeque<Propagator>);

impl PropagatorBuffer {
    pub fn pv_at(&self, stamp: Duration, system: &OrbitalSystem) -> Option<PV> {
        if stamp == self.trange()?.1 {
            let prop = self.0.back()?;
            return system.global_transform(prop);
        }
        let i1 = self.0.iter().position(|e| e.epoch() > stamp)?;
        let l = self.0.get(i1 - 1)?;
        let r = self.0.get(i1)?;
        let p1 = system.global_transform(l)?;
        let p2 = system.global_transform(r)?;
        let s = (stamp - l.epoch()).as_secs_f32() / (r.epoch() - l.epoch()).as_secs_f32();
        Some(p1.lerp(&p2, s))
    }

    pub fn trange(&self) -> Option<(Duration, Duration)> {
        let (f, l) = self.0.front().zip(self.0.back())?;
        Some((f.epoch(), l.epoch()))
    }

    pub fn request_until(&mut self, stamp: Duration) {
        todo!()
    }
}

impl Propagate for Propagator {
    fn propagate_to(&mut self, epoch: Duration, state: &OrbitalSystem) {
        match self {
            Propagator::NBody(nb) => nb.propagate_to(&state.bodies(), epoch),
            Propagator::Kepler(k) => k.propagate_to(epoch),
            Propagator::Fixed(_, _) => (),
        };
    }

    fn next(&self, state: &OrbitalSystem) -> Self {
        match self {
            Propagator::NBody(nb) => Propagator::NBody(nb.next(&state.bodies())),
            Propagator::Kepler(k) => Propagator::Kepler(k.next()),
            Propagator::Fixed(_, _) => *self,
        }
    }

    fn epoch(&self) -> Duration {
        match self {
            Propagator::NBody(n) => n.epoch(),
            Propagator::Kepler(k) => k.epoch(),
            Propagator::Fixed(_, o) => Duration::default(),
        }
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
    pub epoch: Duration,
    pub dt: Duration,
    pub pos: Vec2,
    pub vel: Vec2,
}

impl NBodyPropagator {
    pub fn new(epoch: Duration, pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        NBodyPropagator {
            epoch,
            dt: Duration::from_millis(100),
            pos: pos.into(),
            vel: vel.into(),
        }
    }

    pub fn initial(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        NBodyPropagator {
            epoch: Duration::default(),
            dt: Duration::from_millis(100),
            pos: pos.into(),
            vel: vel.into(),
        }
    }

    pub fn pv(&self) -> PV {
        PV::new(self.pos, self.vel)
    }

    pub fn epoch(&self) -> Duration {
        self.epoch
    }

    pub fn next(&self, bodies: &[(ObjectId, Vec2, Body)]) -> Self {
        let mut copy = *self;
        copy.step(bodies);
        copy
    }

    pub fn step(&mut self, bodies: &[(ObjectId, Vec2, Body)]) {
        let steps = 1; // (delta.as_secs_f32() * self.vel.length() / 3.0).ceil() as u32;
        let dt = self.dt.as_secs_f32() / steps as f32;

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
                let a = compute_a_at(self.pos);
                let v_half = self.vel + a * 0.5 * dt as f32;
                self.pos += v_half * dt as f32;
                let a2 = compute_a_at(self.pos);
                self.vel = v_half + a2 * 0.5 * dt as f32;
            }
        });

        self.epoch += self.dt;
    }

    pub fn propagate_to(&mut self, bodies: &[(ObjectId, Vec2, Body)], epoch: Duration) {
        while self.epoch < epoch {
            self.step(&bodies);
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

    pub fn from_pv(pos: Vec2, vel: Vec2, body: Body, primary: ObjectId) -> Self {
        let orbit = Orbit::from_pv(pos, vel, body);
        KeplerPropagator {
            epoch: Duration::default(),
            primary,
            orbit,
        }
    }

    pub fn propagate_to(&mut self, epoch: Duration) {
        let delta = epoch - self.epoch;
        let n = self.orbit.mean_motion();
        let m = self.orbit.mean_anomaly();
        let m2 = m + delta.as_secs_f32() * n;
        self.orbit.true_anomaly = anomaly_m2t(self.orbit.eccentricity, m2).unwrap_or(f32::NAN);
        self.epoch = epoch;
    }

    pub fn next(&self) -> Self {
        let mut copy = *self;
        copy.propagate_to(copy.epoch + Duration::from_millis(50));
        copy
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
