use crate::canonical::*;
use crate::propagator::*;
use bevy::math::Vec2;
use rand::Rng;
use std::ops::Add;
use std::time::Duration;

pub fn rand(min: f32, max: f32) -> f32 {
    rand::thread_rng().gen_range(min..max)
}

pub fn randvec(min: f32, max: f32) -> Vec2 {
    let rot = Vec2::from_angle(rand(0.0, std::f32::consts::PI * 2.0));
    let mag = rand(min, max);
    rot.rotate(Vec2::new(mag, 0.0))
}

pub fn rotate(v: Vec2, angle: f32) -> Vec2 {
    Vec2::from_angle(angle).rotate(v)
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
    pub const fn new(radius: f32, mass: f32, soi: f32) -> Self {
        Body { radius, mass, soi }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Orbit {
    pub eccentricity: f32,
    pub semi_major_axis: f32,
    pub arg_periapsis: f32,
    pub retrograde: bool,
    pub primary_mass: f32,
}

impl Orbit {
    pub fn is_nan(&self) -> bool {
        self.eccentricity.is_nan() || self.semi_major_axis.is_nan() || self.arg_periapsis.is_nan()
    }

    pub fn from_pv(r: impl Into<Vec2>, v: impl Into<Vec2>, mass: f32) -> Self {
        let mu = mass * GRAVITATIONAL_CONSTANT;
        let r3 = r.into().extend(0.0);
        let v3 = v.into().extend(0.0);
        let h = r3.cross(v3);
        let e = v3.cross(h) / mu - r3 / r3.length();
        let arg_periapsis: f32 = f32::atan2(e.y, e.x);
        let semi_major_axis: f32 = h.length_squared() / (mu * (1.0 - e.length_squared()));
        let mut true_anomaly = f32::acos(e.dot(r3) / (e.length() * r3.length()));
        if r3.dot(v3) < 0.0 {
            true_anomaly = 2.0 * std::f32::consts::PI - true_anomaly;
        }

        Orbit {
            eccentricity: e.length(),
            semi_major_axis,
            arg_periapsis,
            retrograde: h.z < 0.0,
            primary_mass: mass,
        }
    }

    pub const fn circular(radius: f32, ta: f32, mass: f32) -> Self {
        Orbit {
            eccentricity: 0.0,
            semi_major_axis: radius,
            arg_periapsis: 0.0,
            retrograde: false,
            primary_mass: mass,
        }
    }

    pub fn prograde_at(&self, true_anomaly: f32) -> Vec2 {
        let fpa = self.flight_path_angle_at(true_anomaly);
        Vec2::from_angle(fpa).rotate(self.tangent_at(true_anomaly))
    }

    pub fn flight_path_angle_at(&self, true_anomaly: f32) -> f32 {
        -(self.eccentricity * true_anomaly.sin())
            .atan2(1.0 + self.eccentricity * true_anomaly.cos())
    }

    pub fn tangent_at(&self, true_anomaly: f32) -> Vec2 {
        let n = self.normal_at(true_anomaly);
        let angle = match self.retrograde {
            true => -std::f32::consts::PI / 2.0,
            false => std::f32::consts::PI / 2.0,
        };
        Vec2::from_angle(angle).rotate(n)
    }

    pub fn normal_at(&self, true_anomaly: f32) -> Vec2 {
        self.position_at(true_anomaly).normalize()
    }

    pub fn semi_latus_rectum(&self) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
    }

    pub fn angular_momentum(&self) -> f32 {
        (self.mu() * self.semi_latus_rectum()).sqrt()
    }

    pub fn radius_at(&self, true_anomaly: f32) -> f32 {
        self.semi_major_axis * (1.0 - self.eccentricity.powi(2))
            / (1.0 + self.eccentricity * f32::cos(true_anomaly))
    }

    pub fn period(&self) -> Duration {
        let t = 2.0 * std::f32::consts::PI * (self.semi_major_axis.powi(3) / (self.mu())).sqrt();
        Duration::from_secs_f32(t)
    }

    pub fn ta_at_time(&self, mut stamp: Duration) -> f32 {
        let p = self.period();
        while stamp > p {
            stamp -= p;
        }
        let n = self.mean_motion();
        let m = stamp.as_secs_f32() * n;
        anomaly_m2t(self.eccentricity, m).unwrap_or(f32::NAN)
    }

    pub fn pv_at_time(&self, stamp: Duration) -> PV {
        let ta = self.ta_at_time(stamp);
        self.pv_at(ta)
    }

    pub fn pv_at(&self, true_anomaly: f32) -> PV {
        PV::new(
            self.position_at(true_anomaly),
            self.velocity_at(true_anomaly),
        )
    }

    pub fn position_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        let angle = match self.retrograde {
            false => true_anomaly,
            true => -true_anomaly,
        };
        Vec2::from_angle(angle + self.arg_periapsis) * r
    }

    pub fn velocity_at(&self, true_anomaly: f32) -> Vec2 {
        let r = self.radius_at(true_anomaly);
        let v = (self.mu() * (2.0 / r - 1.0 / self.semi_major_axis)).sqrt();
        let h = self.angular_momentum();
        let cosfpa = h / (r * v);
        let sinfpa = cosfpa * self.eccentricity * true_anomaly.sin()
            / (1.0 + self.eccentricity * true_anomaly.cos());
        let n = self.normal_at(true_anomaly);
        let t = self.tangent_at(true_anomaly);
        v * (t * cosfpa + n * sinfpa)
    }

    pub fn periapsis(&self) -> Vec2 {
        self.position_at(0.0)
    }

    pub fn apoapsis(&self) -> Vec2 {
        self.position_at(std::f32::consts::PI)
    }

    pub fn mean_motion(&self) -> f32 {
        (self.mu() / self.semi_major_axis.abs().powi(3)).sqrt()
    }

    pub fn mean_anomaly_at(&self, true_anomaly: f32) -> f32 {
        anomaly_t2m(self.eccentricity, true_anomaly)
    }

    pub fn mu(&self) -> f32 {
        GRAVITATIONAL_CONSTANT * self.primary_mass
    }
}

pub fn gravity_accel(body: Body, body_center: Vec2, sample: Vec2) -> Vec2 {
    let r: Vec2 = body_center - sample;
    let rsq = r.length_squared().clamp(body.radius.powi(2), std::f32::MAX);
    let a = GRAVITATIONAL_CONSTANT * body.mass / rsq;
    a * r.normalize()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    pub prop: Propagator,
    pub body: Option<Body>,
}

impl Object {
    pub fn new(id: ObjectId, prop: impl Into<Propagator>, body: Option<Body>) -> Self {
        Object {
            id,
            prop: prop.into(),
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrbitalSystem {
    pub iter: usize,
    pub epoch: Duration,
    pub objects: Vec<Object>,
    next_id: i64,
    pub units: CanonicalUnits,
}

impl Default for OrbitalSystem {
    fn default() -> Self {
        OrbitalSystem {
            iter: 0,
            epoch: Duration::default(),
            objects: Vec::default(),
            next_id: 0,
            units: earth_moon_canonical_units(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PV {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl PV {
    pub fn new(pos: impl Into<Vec2>, vel: impl Into<Vec2>) -> Self {
        PV {
            pos: pos.into(),
            vel: vel.into(),
        }
    }
}

impl Add for PV {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        PV::new(self.pos + other.pos, self.vel + other.vel)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OrbitalEvent {
    LookupFailure(ObjectId),
    NumericalError(ObjectId),
    Collision(Vec2, ObjectId, Option<ObjectId>),
    Escaped(Vec2, ObjectId),
}

impl OrbitalSystem {
    pub fn add_object(&mut self, prop: impl Into<Propagator>, body: Option<Body>) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;

        let p = prop.into();

        self.objects.push(Object::new(id, p, body));
        id
    }

    pub fn has_object(&self, id: ObjectId) -> bool {
        self.objects.iter().find(|o| o.id == id).is_some()
    }

    pub fn min_id(&self) -> Option<ObjectId> {
        self.objects.iter().map(|o| o.id).min()
    }

    pub fn max_id(&self) -> Option<ObjectId> {
        self.objects.iter().map(|o| o.id).max()
    }

    pub fn lookup(&self, o: ObjectId) -> Option<Object> {
        self.objects.iter().find(|m| m.id == o).map(|m| m.clone())
    }

    pub fn lookup_ref(&self, o: ObjectId) -> Option<&Object> {
        self.objects.iter().find(|m| m.id == o)
    }

    pub fn lookup_mut(&mut self, o: ObjectId) -> Option<&mut Object> {
        self.objects.iter_mut().find(|m| m.id == o)
    }

    pub fn transform_from_id(&self, id: Option<ObjectId>, stamp: Duration) -> Option<PV> {
        if let Some(i) = id {
            let obj = self.lookup(i)?;
            self.global_transform(&obj.prop, stamp)
        } else {
            Some(PV::default())
        }
    }

    pub fn global_transform(&self, prop: &impl Propagate, stamp: Duration) -> Option<PV> {
        if let Some(rel) = prop.relative_to() {
            let obj = self.lookup(rel)?;
            let rel = self.global_transform(&obj.prop, stamp)?;
            Some(prop.pv_at(stamp)? + rel)
        } else {
            Some(prop.pv_at(stamp)?)
        }
    }

    pub fn bodies(&self) -> Vec<(ObjectId, Vec2, Body)> {
        self.objects
            .iter()
            .filter_map(|o| {
                Some((
                    o.id,
                    self.global_transform(&o.prop, self.epoch)?.pos,
                    o.body?,
                ))
            })
            .collect()
    }

    pub fn gravity_at(&self, pos: Vec2) -> Vec2 {
        self.bodies()
            .iter()
            .map(|(_, c, b)| gravity_accel(*b, *c, pos))
            .sum()
    }

    pub fn potential_at(&self, pos: Vec2) -> f32 {
        self.bodies()
            .iter()
            .map(|(_, c, b)| {
                let r = (c - pos).length();
                if r < b.radius {
                    return 0.0;
                }
                -(b.mass * GRAVITATIONAL_CONSTANT) / r
            })
            .sum()
    }

    pub fn primary_body_at(&self, pos: Vec2, exclude: Option<ObjectId>) -> Option<Object> {
        let mut ret = self
            .objects
            .iter()
            .filter_map(|o| {
                if Some(o.id) == exclude {
                    return None;
                }
                let soi = o.body?.soi;
                let bpos = self.global_transform(&o.prop, self.epoch)?;
                let d = bpos.pos.distance(pos);
                if d > soi {
                    return None;
                }
                Some((o.clone(), soi))
            })
            .collect::<Vec<_>>();

        ret.sort_by(|(_, l), (_, r)| l.partial_cmp(r).unwrap());
        ret.first().map(|(o, _)| o.clone())
    }

    pub fn barycenter(&self) -> (Vec2, f32) {
        let bodies = self.bodies();
        let total_mass: f32 = bodies.iter().map(|(_, _, b)| b.mass).sum();
        (
            bodies.iter().map(|(_, p, b)| p * b.mass).sum::<Vec2>() / total_mass,
            total_mass,
        )
    }

    pub fn reparent_patched_conics(&mut self) {
        let new_kepler: Vec<_> = self
            .objects
            .iter()
            .filter_map(|obj| {
                match &obj.prop {
                    Propagator::Kepler(k) => {
                        let child_pv = self.global_transform(&obj.prop, self.epoch)?;
                        let primary = self.primary_body_at(child_pv.pos, Some(obj.id))?;
                        if primary.id == k.primary {
                            return None;
                        }
                        let primary_pv = self.global_transform(&primary.prop, self.epoch)?;
                        // TODO math operators for PV?
                        let ds = child_pv.pos - primary_pv.pos;
                        let dv = child_pv.vel - primary_pv.vel;
                        let orbit = Orbit::from_pv(ds, dv, primary.body?.mass);
                        let mut new_prop = *k;
                        new_prop.orbit = orbit;
                        new_prop.primary = primary.id;
                        Some((obj.id, new_prop))
                    }
                    _ => None,
                }
            })
            .collect();

        for (id, prop) in new_kepler.iter() {
            if let Some(obj) = self.lookup_mut(*id) {
                obj.prop = (*prop).into();
            }
        }
    }
}

pub fn generate_square_lattice(center: Vec2, w: i32, step: usize) -> Vec<Vec2> {
    let mut ret = vec![];
    for x in (-w..w).step_by(step) {
        for y in (-w..w).step_by(step) {
            ret.push(center + Vec2::new(x as f32, y as f32));
        }
    }
    ret
}

pub fn generate_circular_log_lattice(center: Vec2, rmin: f32, rmax: f32) -> Vec<Vec2> {
    // this isn't actually log, but I'm lazy
    let mut ret = vec![];

    let mut r = rmin;
    let mut dr = 30.0;

    while r < rmax {
        let circ = 2.0 * std::f32::consts::PI * r;
        let mut pts = (circ / dr).ceil() as u32;
        while pts % 8 > 0 {
            pts += 1; // yeah this is stupid
        }
        for i in 0..pts {
            let a = 2.0 * std::f32::consts::PI * i as f32 / pts as f32;
            let x = a.cos();
            let y = a.sin();
            ret.push(center + Vec2::new(x, y) * r);
        }

        r += dr;
        dr *= 1.1;
    }

    ret
}

pub fn synodic_period(t1: Duration, t2: Duration) -> Option<Duration> {
    if t1 == t2 {
        return None;
    }

    let s1 = t1.as_secs_f32();
    let s2 = t2.as_secs_f32();

    Some(Duration::from_secs_f32(s1 * s2 / (s2 - s1).abs()))
}
