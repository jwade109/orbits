use crate::orbiter::*;
use crate::orbits::sparse_orbit::{Body, SparseOrbit};
use crate::pv::PV;
use glam::f32::Vec2;
use rand::Rng;
use std::ops::{Add, AddAssign, Div, Mul, Rem, Sub, SubAssign};

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

pub fn cross2d(a: Vec2, b: Vec2) -> f32 {
    a.extend(0.0).cross(b.extend(0.0)).z
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn apply<T: Copy, R>(x: &Vec<T>, func: impl Fn(T) -> R) -> Vec<R> {
    x.iter().map(|x| func(*x)).collect()
}

pub fn linspace(a: f32, b: f32, n: usize) -> Vec<f32> {
    if n < 2 {
        return vec![a];
    }
    if n == 2 {
        return vec![a, b];
    }
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            lerp(a, b, t)
        })
        .collect()
}

pub fn gravity_accel(body: Body, body_center: Vec2, sample: Vec2) -> Vec2 {
    let r: Vec2 = body_center - sample;
    let rsq = r.length_squared().clamp(body.radius.powi(2), std::f32::MAX);
    let a = body.mu() / rsq;
    a * r.normalize()
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nanotime(pub i64);

impl Nanotime {
    pub const PER_SEC: i64 = 1000000000;
    pub const PER_MILLI: i64 = 1000000;

    pub fn to_secs(&self) -> f32 {
        self.0 as f32 / Nanotime::PER_SEC as f32
    }

    pub fn to_secs_f64(&self) -> f64 {
        self.0 as f64 / Nanotime::PER_SEC as f64
    }

    pub fn to_parts(&self) -> (i64, i64) {
        (self.0 % Nanotime::PER_SEC, self.0 / Nanotime::PER_SEC)
    }

    pub fn secs(s: i64) -> Self {
        Nanotime(s * Nanotime::PER_SEC)
    }

    pub fn millis(ms: i64) -> Self {
        Nanotime(ms * Nanotime::PER_MILLI)
    }

    pub fn secs_f32(s: f32) -> Self {
        Nanotime((s * Nanotime::PER_SEC as f32) as i64)
    }

    pub fn ceil(&self, order: i64) -> Self {
        Self((self.0 + order) - (self.0 % order))
    }

    pub fn floor(&self, order: i64) -> Self {
        Self(self.0 - (self.0 % order))
    }
}

impl core::fmt::Debug for Nanotime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let disp = self.0.abs();
        if self.0 >= 0 {
            write!(f, "{}.{:09}", disp / 1000000000, disp % 1000000000)
        } else {
            write!(f, "-{}.{:09}", disp / 1000000000, disp % 1000000000)
        }
    }
}

impl Add for Nanotime {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Nanotime(self.0 + other.0)
    }
}

impl AddAssign for Nanotime {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl Sub for Nanotime {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        // TODO disallow wrapping?
        Nanotime(self.0.wrapping_sub(other.0))
    }
}

impl SubAssign for Nanotime {
    fn sub_assign(&mut self, rhs: Self) {
        let res = self.sub(rhs);
        *self = res;
    }
}

impl Mul<i64> for Nanotime {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self {
        Self(self.0 * rhs)
    }
}

impl Mul<f32> for Nanotime {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self((self.0 as f32 * rhs) as i64)
    }
}

impl Div<i64> for Nanotime {
    type Output = Self;
    fn div(self, rhs: i64) -> Self {
        Self(self.0 / rhs)
    }
}

impl Rem<Nanotime> for Nanotime {
    type Output = Self;
    fn rem(self, rhs: Nanotime) -> Self::Output {
        Nanotime(self.0 % rhs.0)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectType {
    Orbiter,
    System,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectIdTracker(ObjectId);

impl ObjectIdTracker {
    pub fn new() -> Self {
        ObjectIdTracker(ObjectId(0))
    }

    pub fn next(&mut self) -> ObjectId {
        let ret = self.0;
        self.0 .0 += 1;
        ret
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Collide(ObjectId),
    Escape(ObjectId),
    Encounter(ObjectId),
    Maneuver(Maneuver),
    NumericalError,
}

impl PartialEq for EventType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EventType::Collide(o), EventType::Collide(p)) => o == p,
            (EventType::Escape(o), EventType::Escape(p)) => o == p,
            (EventType::Encounter(o), EventType::Encounter(p)) => o == p,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectLookup {
    pub object: Orbiter,
    pub level: u32,
    pub local_pv: PV,
    pub frame_pv: PV,
    pub otype: ObjectType,
    pub parent: ObjectId,
    pub body: Option<Body>,
}

impl ObjectLookup {
    pub fn pv(&self) -> PV {
        self.local_pv + self.frame_pv
    }
}

#[derive(Debug, Clone)]
pub struct OrbitalTree {
    pub objects: Vec<Orbiter>,
    pub system: PlanetarySystem,
}

#[derive(Debug, Clone)]
pub struct PlanetarySystem {
    pub id: ObjectId,
    pub name: String,
    pub body: Body,
    pub subsystems: Vec<(SparseOrbit, PlanetarySystem)>,
}

impl PlanetarySystem {
    pub fn new(id: ObjectId, name: impl Into<String>, body: Body) -> Self {
        PlanetarySystem {
            id,
            name: name.into(),
            body,
            subsystems: vec![],
        }
    }

    pub fn orbit(&mut self, orbit: SparseOrbit, planets: PlanetarySystem) {
        self.subsystems.push((orbit, planets));
    }

    fn lookup_inner(
        &self,
        id: ObjectId,
        stamp: Nanotime,
        wrt: PV,
        parent_id: Option<ObjectId>,
    ) -> Option<(Body, PV, Option<ObjectId>, &PlanetarySystem)> {
        if self.id == id {
            return Some((self.body, wrt, parent_id, self));
        }

        for (orbit, pl) in &self.subsystems {
            let pv = orbit.pv_at_time(stamp);
            let ret = pl.lookup_inner(id, stamp, wrt + pv, Some(self.id));
            if let Some(r) = ret {
                return Some(r);
            }
        }

        None
    }

    pub fn lookup(
        &self,
        id: ObjectId,
        stamp: Nanotime,
    ) -> Option<(Body, PV, Option<ObjectId>, &PlanetarySystem)> {
        self.lookup_inner(id, stamp, PV::zero(), None)
    }
}

#[derive(Debug, Clone)]
pub struct RemovalInfo {
    pub stamp: Nanotime,
    pub reason: EventType,
    pub orbit: SparseOrbit,
}

impl OrbitalTree {
    pub fn new(system: &PlanetarySystem) -> Self {
        OrbitalTree {
            objects: vec![],
            system: system.clone(),
        }
    }

    pub fn propagate_to(
        &mut self,
        stamp: Nanotime,
        future_dur: Nanotime,
    ) -> Vec<(ObjectId, Option<RemovalInfo>)> {
        for obj in &mut self.objects {
            let _ = obj.propagate_to(stamp, future_dur, &self.system);
        }

        let mut info = vec![];

        self.objects.retain(|o| {
            if o.propagator_at(stamp).is_none() {
                let reason = o.props().last().map(|p| RemovalInfo {
                    stamp: p.end,
                    reason: p.event.unwrap_or(EventType::NumericalError),
                    orbit: p.orbit.clone(),
                });
                info.push((o.id, reason));
                false
            } else {
                true
            }
        });

        info
    }

    pub fn add_object(
        &mut self,
        id: ObjectId,
        parent: ObjectId,
        orbit: SparseOrbit,
        stamp: Nanotime,
    ) {
        self.objects.push(Orbiter::new(id, parent, orbit, stamp));
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<()> {
        self.objects
            .remove(self.objects.iter().position(|o| o.id == id)?);
        Some(())
    }

    pub fn orbiter_lookup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup> {
        self.objects.iter().find_map(|o| {
            if o.id != id {
                return None;
            }

            let prop = o.propagator_at(stamp)?;

            let (_, frame_pv, _, _) = self.system.lookup(prop.parent, stamp)?;

            Some(ObjectLookup {
                object: o.clone(),
                level: 0,
                local_pv: prop.pv(stamp)?,
                frame_pv,
                otype: ObjectType::Orbiter,
                parent: ObjectId(0),
                body: None,
            })
        })
    }
}

pub fn potential_at(planet: &PlanetarySystem, pos: Vec2, stamp: Nanotime) -> f32 {
    let r = pos.length().clamp(10.0, std::f32::MAX);
    let mut ret = -planet.body.mu() / r;
    for (orbit, pl) in &planet.subsystems {
        let pv = orbit.pv_at_time(stamp);
        ret += potential_at(pl, pos - pv.pos, stamp);
    }
    ret
}
