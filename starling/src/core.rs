use crate::{orbit::*, planning::get_future_path};
use bevy::{math::Vec2, utils::tracing::Event};
use rand::Rng;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

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

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
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
    let a = GRAVITATIONAL_CONSTANT * body.mass / rsq;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

impl Add for ObjectId {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

#[derive(Debug, Clone)]
pub struct OrbitalSystem {
    pub primary: Body,
    pub objects: Vec<(ObjectId, Orbit)>,
    pub subsystems: Vec<(ObjectId, Orbit, OrbitalSystem)>,
    pub high_water_mark: ObjectId,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PV {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl PV {
    pub fn zero() -> Self {
        PV {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
        }
    }

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

impl Sub for PV {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        PV::new(self.pos - other.pos, self.vel - other.vel)
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
pub struct OrbitalEvent {
    pub target: ObjectId,
    pub stamp: Nanotime,
    pub etype: EventType,
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Collide,
    Escape,
    Encounter(ObjectId),
    Maneuver(Vec2),
}

impl OrbitalEvent {
    pub fn collision(target: ObjectId, stamp: Nanotime) -> Self {
        OrbitalEvent {
            target,
            stamp,
            etype: EventType::Collide,
        }
    }

    pub fn escape(target: ObjectId, stamp: Nanotime) -> Self {
        OrbitalEvent {
            target,
            stamp,
            etype: EventType::Escape,
        }
    }

    pub fn encounter(target: ObjectId, body: ObjectId, stamp: Nanotime) -> Self {
        OrbitalEvent {
            target,
            stamp,
            etype: EventType::Encounter(body),
        }
    }

    pub fn maneuver(target: ObjectId, dv: Vec2, stamp: Nanotime) -> Self {
        OrbitalEvent {
            target,
            stamp,
            etype: EventType::Maneuver(dv),
        }
    }
}

impl OrbitalSystem {
    pub fn new(body: Body) -> Self {
        OrbitalSystem {
            primary: body,
            objects: Vec::default(),
            subsystems: vec![],
            high_water_mark: ObjectId(0),
        }
    }

    pub fn apply(&mut self, event: OrbitalEvent) -> Option<()> {
        match event.etype {
            EventType::Collide => {
                self.remove_object(event.target);
            }
            EventType::Escape => {
                // TODO, this should emit the object
                self.remove_object(event.target);
            }
            EventType::Encounter(pri) => {
                // TODO!
                // let (_, spv) = self.lookup_subsystem(pri, event.stamp)?;
                // let pv = self.lookup_orbiter(event.target)?.pv_at_time(event.stamp);
                // let rel = pv - spv;
            }
            EventType::Maneuver(dv) => {
                let dpv = PV::new(Vec2::ZERO, dv);
                let m = self.primary.mass;
                let orbit = self.lookup_orbiter_mut(event.target)?;
                let pv = orbit.pv_at_time(event.stamp) + dpv;
                *orbit = Orbit::from_pv(pv.pos, pv.vel, m, event.stamp);
            }
        };
        Some(())
    }

    pub fn add_object(&mut self, id: ObjectId, orbit: Orbit) {
        self.objects.push((id, orbit));
        self.high_water_mark.0 = self.high_water_mark.0.max(id.0)
    }

    pub fn remove_object(&mut self, id: ObjectId) {
        self.objects.retain(|(o, _)| *o != id)
    }

    pub fn add_subsystem(&mut self, id: ObjectId, orbit: Orbit, subsys: OrbitalSystem) {
        self.subsystems.push((id, orbit, subsys));
        self.high_water_mark.0 = self.high_water_mark.0.max(id.0)
    }

    pub fn has_object(&self, id: ObjectId) -> bool {
        self.objects.iter().find(|o| o.0 == id).is_some()
    }

    pub fn otype(&self, o: ObjectId) -> Option<ObjectType> {
        if self.lookup_orbiter(o).is_some() {
            Some(ObjectType::Orbiter)
        } else if self.lookup_system(o).is_some() {
            Some(ObjectType::System)
        } else {
            None
        }
    }

    fn lookup_orbiter(&self, o: ObjectId) -> Option<&Orbit> {
        self.objects
            .iter()
            .find_map(|(id, orbit)| if *id == o { Some(orbit) } else { None })
    }

    fn lookup_system(&self, o: ObjectId) -> Option<(&Orbit, &OrbitalSystem)> {
        self.subsystems
            .iter()
            .find_map(|(id, orbit, sys)| if *id == o { Some((orbit, sys)) } else { None })
    }

    pub fn lookup(&self, o: ObjectId) -> Option<&Orbit> {
        self.lookup_orbiter(o)
            .or_else(|| Some(self.lookup_system(o)?.0))
    }

    fn lookup_orbiter_mut(&mut self, o: ObjectId) -> Option<&mut Orbit> {
        self.objects
            .iter_mut()
            .find_map(|(id, orbit)| if *id == o { Some(orbit) } else { None })
    }

    pub fn potential_at(&self, pos: Vec2, stamp: Nanotime) -> f32 {
        let r = pos.length().clamp(10.0, std::f32::MAX);
        let mut ret = -(self.primary.mass * GRAVITATIONAL_CONSTANT) / r;
        for (_, orbit, sys) in &self.subsystems {
            let pv = orbit.pv_at_time(stamp);
            ret += sys.potential_at(pos - pv.pos, stamp);
        }
        ret
    }

    pub fn barycenter(&self) -> (Vec2, f32) {
        (Vec2::ZERO, self.primary.mass)
        // TODO sum subsystems
    }

    pub fn rebalance(&mut self, stamp: Nanotime) {
        self.remove_collided(stamp);
        self.eject_escaped(stamp);
        self.reparent(stamp);
    }

    pub fn remove_collided(&mut self, stamp: Nanotime) {
        for (_, _, subsys) in &mut self.subsystems {
            subsys.remove_collided(stamp);
        }

        self.objects.retain(|(_, orbit)| {
            if orbit.periapsis_r() > self.primary.radius {
                return true;
            }
            let pv = orbit.pv_at_time(stamp);
            let r2 = pv.pos.length_squared();
            r2 > self.primary.radius.powi(2)
        });
    }

    pub fn eject_escaped(&mut self, stamp: Nanotime) -> Vec<(ObjectId, PV)> {
        let ejecta = self
            .subsystems
            .iter_mut()
            .map(|(_, orb, s)| {
                let syspv = orb.pv_at_time(stamp);
                s.eject_escaped(stamp)
                    .iter()
                    .map(|(id, pv)| (*id, *pv + syspv))
                    .collect::<Vec<_>>()
            })
            .flatten()
            .collect::<Vec<_>>();

        for (id, pv) in ejecta {
            let orbit = Orbit::from_pv(pv.pos, pv.vel, self.primary.mass, stamp);
            self.add_object(id, orbit);
        }

        let mut ret = vec![];
        self.objects.retain(|(id, orbit)| -> bool {
            if orbit.eccentricity < 1.0 && orbit.apoapsis_r() < self.primary.soi {
                return true;
            }
            let pv = orbit.pv_at_time(stamp);
            let keep = pv.pos.length_squared() <= self.primary.soi.powi(2);
            if !keep {
                ret.push((*id, orbit.pv_at_time(stamp)));
            }
            keep
        });
        ret
    }

    pub fn reparent(&mut self, stamp: Nanotime) {
        // TODO fix this!
        let mut to_remove = vec![];
        for (id, orbit) in &mut self.objects {
            let pv = orbit.pv_at_time(stamp);
            let new = self
                .subsystems
                .iter_mut()
                .filter_map(|(sysid, sysorb, sys)| {
                    let syspv = sysorb.pv_at_time(stamp);
                    let rel = pv - syspv;
                    if rel.pos.length_squared() < sys.primary.soi.powi(2) {
                        Some((sys, rel))
                    } else {
                        None
                    }
                })
                .next();
            if let Some((nsys, rel)) = new {
                let neworb = Orbit::from_pv(rel.pos, rel.vel, nsys.primary.mass, stamp);
                nsys.add_object(*id, neworb);
                to_remove.push(*id);
            }
        }
        self.objects.retain(|(id, _)| !to_remove.contains(id));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AABB(pub Vec2, pub Vec2);

impl AABB {
    pub fn from_center(c: Vec2, span: Vec2) -> Self {
        let low = c - span / 2.0;
        let hi = c + span / 2.0;
        Self(low, hi)
    }

    pub fn center(&self) -> Vec2 {
        (self.0 + self.1) / 2.0
    }

    pub fn span(&self) -> Vec2 {
        self.1 - self.0
    }

    pub fn to_uniform(&self, p: Vec2) -> Vec2 {
        let u = p - self.0;
        let s = self.span();
        u / s
    }

    pub fn from_uniform(&self, u: Vec2) -> Vec2 {
        u * self.span() + self.0
    }

    pub fn map(from: Self, to: Self, p: Vec2) -> Vec2 {
        let u = from.to_uniform(p);
        to.from_uniform(u)
    }
}
