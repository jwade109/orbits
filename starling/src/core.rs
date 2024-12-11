use bevy::math::Vec2;
use rand::Rng;
use std::time::Duration;

use crate::transforms::*;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
}

pub fn anomaly_e2m(ecc: f32, eccentric_anomaly: f32) -> f32 {
    eccentric_anomaly - ecc * f32::sin(eccentric_anomaly)
}

pub fn anomaly_m2e(ecc: f32, mean_anomaly: f32) -> Option<f32> {
    let max_error = 1E-6;
    let max_iters = 1000;

    let mut e = mean_anomaly;

    for _ in 0..max_iters {
        e = e - (mean_anomaly - e + ecc * e.sin()) / (ecc * e.cos() - 1.0);
        if (mean_anomaly - e + ecc * e.sin()).abs() < max_error {
            return Some(e);
        }
    }

    None
}

pub fn anomaly_t2e(ecc: f32, true_anomaly: f32) -> f32 {
    f32::atan2(
        f32::sin(true_anomaly) * (1.0 - ecc.powi(2)).sqrt(),
        f32::cos(true_anomaly) + ecc,
    )
}

pub fn anomaly_e2t(ecc: f32, eccentric_enomaly: f32) -> f32 {
    f32::atan2(
        f32::sin(eccentric_enomaly) * (1.0 - ecc.powi(2)).sqrt(),
        f32::cos(eccentric_enomaly) - ecc,
    )
}

pub fn anomaly_t2m(ecc: f32, true_anomaly: f32) -> f32 {
    anomaly_e2m(ecc, anomaly_t2e(ecc, true_anomaly))
}

pub fn anomaly_m2t(ecc: f32, mean_anomaly: f32) -> Option<f32> {
    anomaly_m2e(ecc, mean_anomaly).map(|e| anomaly_e2t(ecc, e))
}

pub const GRAVITATIONAL_CONSTANT: f32 = 12000.0;

#[derive(Debug, Clone, Copy)]
pub struct Body {
    pub radius: f32,
    pub mass: f32,
    pub soi: f32,
}

impl Body {
    pub fn mu(&self) -> f32 {
        self.mass * GRAVITATIONAL_CONSTANT
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub eccentricity: f32,
    pub semi_major_axis: f32,
    pub arg_periapsis: f32,
    pub true_anomaly: f32,
    pub body: Body,
}

impl Orbit {
    pub fn from_pv(r: Vec2, v: Vec2, body: Body) -> Self {
        let r3 = r.extend(0.0);
        let v3 = v.extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / body.mu() - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (body.mu() * (1.0 - e.length_squared()));
        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = 2.0 * std::f32::consts::PI - true_anomaly;
        }
        if h.z < 0.0 {
            true_anomaly *= -1.0;
        }

        Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            true_anomaly,
            body,
        }
    }

    pub fn radius_at(&self, true_anomaly: f32) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f32::cos(true_anomaly))
    }

    pub fn period(&self) -> Duration {
        let t =
            2.0 * std::f32::consts::PI * (self.semi_major_axis.powi(3) / (self.body.mu())).sqrt();
        Duration::from_secs_f32(t)
    }

    pub fn pos(&self) -> Vec2 {
        self.position_at(self.true_anomaly)
    }

    pub fn vel(&self) -> Vec2 {
        self.velocity_at(self.true_anomaly)
    }

    pub fn position_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        Vec2::from_angle(true_anomaly + self.arg_periapsis) * r
    }

    pub fn velocity_at(&self, _true_anomaly: f32) -> Vec2 {
        todo!()
    }

    pub fn periapsis(&self) -> Vec2 {
        self.position_at(0.0)
    }

    pub fn apoapsis(&self) -> Vec2 {
        self.position_at(std::f32::consts::PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.body.mu() / self.semi_major_axis.powi(3)).sqrt()
    }

    pub fn mean_anomaly(&self) -> f32 {
        anomaly_t2m(self.eccentricity, self.true_anomaly)
    }
}

pub fn gravity_accel(body: Body, body_center: Vec2, sample: Vec2) -> Vec2 {
    let r: Vec2 = body_center - sample;
    let rsq = r.length_squared().clamp(body.radius.powi(2), std::f32::MAX);
    let a = GRAVITATIONAL_CONSTANT * body.mass / rsq;
    a * r.normalize()
}

pub const EARTH: (Body, Propagator) = (
    Body {
        radius: 63.0,
        mass: 1000.0,
        soi: 15000.0,
    },
    Propagator::Fixed(Vec2::ZERO),
);

pub const LUNA: (Body, Propagator) = (
    Body {
        radius: 22.0,
        mass: 10.0,
        soi: 800.0,
    },
    Propagator::NBody(NBodyPropagator {
        epoch: Duration::new(0, 0),
        pos: Vec2::new(-3800.0, 0.0),
        vel: Vec2::new(0.0, -58.0),
    }),
);

#[derive(Debug, Copy, Clone)]
pub struct NBodyPropagator {
    pub epoch: Duration,
    pub pos: Vec2,
    pub vel: Vec2,
}

impl NBodyPropagator {
    pub fn propagate_to(&mut self, bodies: &[(Vec2, Body)], epoch: Duration) {
        let delta_time = epoch - self.epoch;
        let dt = delta_time.as_secs_f32();

        let steps_per_minute = self.vel.length().clamp(2.0, 10000.0);
        let steps = (steps_per_minute * dt).clamp(5.0, 10000.0) as u32;

        let others = bodies.iter().filter(|(c, _)| {
            *c != self.pos
        }).collect::<Vec<_>>();

        (0..steps).for_each(|_| {
            let a: Vec2 = others
                .iter()
                .map(|(c, b)| -> Vec2 { gravity_accel(*b, *c, self.pos) })
                .sum();

            self.vel += a * dt / steps as f32;
            self.pos += self.vel * dt / steps as f32;
        });

        self.epoch = epoch;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct KeplerPropagator {
    pub epoch: Duration,
    pub primary: ObjectId,
    pub orbit: Orbit,
}

impl KeplerPropagator {
    pub fn from_pv(epoch: Duration, pos: Vec2, vel: Vec2, body: Body, parent: ObjectId) -> Self {
        let orbit = Orbit::from_pv(pos, vel, body);
        KeplerPropagator {
            epoch,
            primary: parent,
            orbit,
        }
    }

    pub fn propagate_to(&mut self, epoch: Duration) {
        let delta = epoch - self.epoch;

        if delta == Duration::default() {
            return;
        }

        let n = self.orbit.mean_motion();
        let m = self.orbit.mean_anomaly();
        let m2 = m + delta.as_secs_f32() * n;
        self.orbit.true_anomaly = anomaly_m2t(self.orbit.eccentricity, m2).unwrap();
        self.epoch = epoch;
    }
}

pub trait Propagate {
    fn epoch(&self) -> Duration;

    fn pos(&self) -> Vec2;

    fn vel(&self) -> Vec2;

    fn propagate_to(&mut self, epoch: Duration, state: &OrbitalSystem);
}

#[derive(Debug, Clone, Copy)]
pub enum Propagator {
    Fixed(Vec2),
    NBody(NBodyPropagator),
    Kepler(KeplerPropagator),
}

impl Propagator {
    pub fn fixed_at(pos: Vec2) -> Self {
        Propagator::Fixed(pos)
    }

    pub fn orbit(&self) -> Option<Orbit> {
        match self {
            Propagator::NBody(nb) => Some(Orbit::from_pv(nb.pos, nb.vel, EARTH.0)),
            Propagator::Kepler(k) => Some(k.orbit),
            Propagator::Fixed(_) => None,
        }
    }
}

impl Propagate for Propagator {

    fn propagate_to(&mut self, epoch: Duration, state: &OrbitalSystem) {
        let bodies = state.bodies();
        match self {
            Propagator::NBody(nb) => nb.propagate_to(&bodies, epoch),
            Propagator::Kepler(k) => k.propagate_to(epoch),
            Propagator::Fixed(_) => (),
        };
    }

    fn epoch(&self) -> Duration {
        match self {
            Propagator::NBody(nb) => nb.epoch,
            Propagator::Kepler(_) => todo!(),
            Propagator::Fixed(_) => Duration::default(),
        }
    }

    fn pos(&self) -> Vec2 {
        match self {
            Propagator::NBody(nb) => nb.pos,
            Propagator::Kepler(k) => k.orbit.pos(),
            Propagator::Fixed(p) => *p,
        }
    }

    fn vel(&self) -> Vec2 {
        match self {
            Propagator::NBody(nb) => nb.vel,
            Propagator::Kepler(k) => k.orbit.vel(),
            Propagator::Fixed(_) => Vec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObjectId(i64);

#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub id: ObjectId,
    pub prop: Propagator,
    pub body: Option<Body>
}

#[derive(Debug, Clone, Default)]
pub struct OrbitalSystem {
    pub epoch: Duration,
    pub objects: Vec<Object>,
    next_id: i64,
}

impl OrbitalSystem {
    pub fn add_object(&mut self, prop: Propagator, body: Option<Body>) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.objects.push(Object { id, prop, body });
        id
    }

    pub fn get_global_position(&self, o: ObjectId) -> Option<Vec2> {
        let prop_lookup = |prop: &Propagator| -> Option<Vec2> {
            return match prop {
                Propagator::Fixed(p) => Some(*p),
                Propagator::NBody(nb) => Some(nb.pos),
                Propagator::Kepler(k) => match self.get_global_position(k.primary) {
                    Some(p) => Some(p + prop.pos()),
                    None => None,
                },
            };
        };

        if let Some(m) = self.lookup(o) {
            return prop_lookup(&m.prop);
        } else {
            None
        }
    }

    pub fn get_dominant_object(&self, pos: Vec2) -> Option<Object> {
        self.objects
            .iter()
            .filter(|m| m.body.is_some())
            .map(|m| {
                (
                    m,
                    self.get_global_position(m.id)
                        .map(|c| gravity_accel(m.body.unwrap(), c, pos).length_squared())
                        .unwrap_or(0.0),
                )
            })
            .max_by(|(_, l), (_, r)| l.total_cmp(&r))
            .map(|(m, _)| *m)
    }

    pub fn lookup(&self, o: ObjectId) -> Option<Object> {
        self.objects.iter().find(|m| m.id == o).map(|m| *m)
    }

    pub fn propagate_to(&mut self, epoch: Duration) {
        let copy = self.clone();
        for m in self.objects.iter_mut() {
            m.prop.propagate_to(epoch, &copy);
        }
    }

    fn bodies(&self) -> Vec<(Vec2, Body)> {
        self.objects
            .iter()
            .filter(|o| o.body.is_some())
            .map(|o| (o.prop.pos(), o.body.unwrap()))
            .collect()
    }
}
