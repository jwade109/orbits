use crate::orbit::*;
use crate::planning::*;
use bevy::math::Vec2;
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

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Object {
    pub id: ObjectId,
    pub orbit: Orbit,
    pub event: Option<OrbitalEvent>,
    pub propagator: Propagator,
}

impl Object {
    pub fn new(id: ObjectId, orbit: Orbit, stamp: Nanotime) -> Self {
        Object {
            id,
            orbit,
            event: None,
            propagator: Propagator::new(stamp),
        }
    }

    pub fn has_update(&self, stamp: Nanotime) -> bool {
        self.event.map_or(false, |e| e.stamp <= stamp)
    }

    pub fn take_event(&mut self, stamp: Nanotime) -> Option<OrbitalEvent> {
        if self.has_update(stamp) {
            self.propagator.reset(stamp);
            self.event.take()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrbitalSystem {
    pub primary: Body,
    pub objects: Vec<Object>,
    pub subsystems: Vec<(Object, OrbitalSystem)>,
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

impl std::fmt::Display for PV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "P({:0.3}, {:0.3}) V({:0.3}, {:0.3})",
            self.pos.x, self.pos.y, self.vel.x, self.vel.y
        )
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
    pub fn new(target: ObjectId, stamp: Nanotime, etype: EventType) -> Self {
        OrbitalEvent {
            target,
            stamp,
            etype,
        }
    }

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

#[derive(Debug, Clone, Copy)]
pub struct ObjectLookup {
    pub object: Object,
    pub local_pv: PV,
    pub frame_pv: PV,
    pub otype: ObjectType,
    pub body: Option<Body>,
}

impl ObjectLookup {
    pub fn pv(&self) -> PV {
        self.local_pv + self.frame_pv
    }
}

impl OrbitalSystem {
    pub fn new(body: Body) -> Self {
        OrbitalSystem {
            primary: body,
            objects: Vec::default(),
            subsystems: vec![],
        }
    }

    pub fn apply(&mut self, event: OrbitalEvent) -> Option<ObjectLookup> {
        match event.etype {
            EventType::Collide => {
                self.remove_object(event.target);
            }
            EventType::Escape => {
                let lup = self.lookup(event.target, event.stamp)?;
                self.remove_object(event.target);
                return Some(lup);
            }
            EventType::Encounter(pri) => {
                let l1 = self.lookup(pri, event.stamp)?;
                let b = l1.body?;
                let l2 = self.lookup(event.target, event.stamp)?;
                let d = l2.pv() - l1.pv();
                let orbit = Orbit::from_pv(d.pos, d.vel, b.mass, event.stamp);
                self.remove_object(event.target);
                let (_, sys) = self.subsystems.iter_mut().find(|(obj, _)| obj.id == pri)?;
                sys.add_object(event.target, orbit, event.stamp)
            }
            EventType::Maneuver(dv) => {
                let dpv = PV::new(Vec2::ZERO, dv);
                let m = self.primary.mass;
                let obj = self.lookup_orbiter_mut(event.target)?;
                let pv = obj.orbit.pv_at_time(event.stamp) + dpv;
                obj.orbit = Orbit::from_pv(pv.pos, pv.vel, m, event.stamp);
                obj.event = None;
            }
        };
        None
    }

    pub fn propagate_to(&mut self, stamp: Nanotime) -> Vec<ObjectLookup> {
        let mut orbits_to_add = vec![];

        for (obj, subsys) in &mut self.subsystems {
            let lups = subsys.propagate_to(stamp);
            let pvsys = obj.orbit.pv_at_time(stamp);
            for lup in lups {
                let pv = pvsys + lup.pv();
                let orbit = Orbit::from_pv(pv.pos, pv.vel, self.primary.mass, stamp);
                orbits_to_add.push((lup.object.id, orbit));
            }
        }

        for (id, orbit) in orbits_to_add {
            self.add_object(id, orbit, stamp)
        }

        let mut to_apply = vec![];
        for obj in &mut self.objects {
            let e = obj.take_event(stamp);
            e.map(|e| to_apply.push(e));
        }

        let mut ret = vec![];
        for e in to_apply {
            let lup = self.apply(e);
            if let Some(lup) = lup {
                ret.push(lup);
            }
        }

        let bodies = self
            .subsystems
            .iter()
            .map(|(obj, sys)| (obj.id, obj.orbit, sys.primary.soi))
            .collect::<Vec<_>>();

        for obj in &mut self.objects {
            while !obj.propagator.calculated_to(stamp + Nanotime::secs(200)) {
                let res =
                    obj.propagator
                        .next(&obj.orbit, self.primary.radius, self.primary.soi, &bodies);
                match res {
                    Err(e) => {
                        dbg!(e);
                    }
                    Ok(Some((t, e))) => {
                        let event = OrbitalEvent::new(obj.id, t, e);
                        obj.event = Some(event);
                    }
                    _ => (),
                }
            }
        }

        ret
    }

    pub fn add_object(&mut self, id: ObjectId, orbit: Orbit, stamp: Nanotime) {
        self.objects.push(Object::new(id, orbit, stamp));
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<Object> {
        if let Some(i) = self.objects.iter().position(|o| o.id == id) {
            return Some(self.objects.swap_remove(i));
        }

        for (_, sys) in &mut self.subsystems {
            let x = sys.remove_object(id);
            if x.is_some() {
                return x;
            }
        }

        None
    }

    pub fn add_subsystem(
        &mut self,
        id: ObjectId,
        orbit: Orbit,
        stamp: Nanotime,
        subsys: OrbitalSystem,
    ) {
        self.subsystems
            .push((Object::new(id, orbit, stamp), subsys));
    }

    pub fn ids(&self) -> Vec<ObjectId> {
        let mut ret: Vec<ObjectId> = self.objects.iter().map(|obj| obj.id).collect();
        for (obj, sys) in &self.subsystems {
            ret.push(obj.id);
            ret.extend_from_slice(&sys.ids());
        }
        ret
    }

    fn lookup_inner(&self, id: ObjectId, stamp: Nanotime, frame_pv: PV) -> Option<ObjectLookup> {
        let find_subsys = || {
            self.subsystems.iter().find_map(|(o, sys)| {
                if o.id != id {
                    return None;
                }

                let local_pv = o.orbit.pv_at_time(stamp);

                Some(ObjectLookup {
                    object: *o,
                    local_pv,
                    frame_pv,
                    otype: ObjectType::System,
                    body: Some(sys.primary),
                })
            })
        };

        let find_object = || {
            self.objects.iter().find_map(|o| {
                if o.id != id {
                    return None;
                }

                Some(ObjectLookup {
                    object: *o,
                    local_pv: o.orbit.pv_at_time(stamp),
                    frame_pv,
                    otype: ObjectType::Orbiter,
                    body: None,
                })
            })
        };

        let find_recurse = || {
            self.subsystems.iter().find_map(|(o, sys)| {
                let pv = o.orbit.pv_at_time(stamp) + frame_pv;
                sys.lookup_inner(id, stamp, pv)
            })
        };

        find_subsys().or_else(find_object).or_else(find_recurse)
    }

    pub fn lookup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup> {
        self.lookup_inner(id, stamp, PV::zero())
    }

    pub fn lookup_orbiter_mut(&mut self, o: ObjectId) -> Option<&mut Object> {
        self.objects
            .iter_mut()
            .find_map(|obj| if obj.id == o { Some(obj) } else { None })
    }

    pub fn potential_at(&self, pos: Vec2, stamp: Nanotime) -> f32 {
        let r = pos.length().clamp(10.0, std::f32::MAX);
        let mut ret = -(self.primary.mass * GRAVITATIONAL_CONSTANT) / r;
        for (obj, sys) in &self.subsystems {
            let pv = obj.orbit.pv_at_time(stamp);
            ret += sys.potential_at(pos - pv.pos, stamp);
        }
        ret
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

    pub fn from_arbitrary(a: Vec2, b: Vec2) -> Self {
        let low = Vec2::new(a.x.min(b.x), a.y.min(b.y));
        let hi = Vec2::new(a.x.max(b.x), a.y.max(b.y));
        Self(low, hi)
    }

    pub fn from_list(plist: &[Vec2]) -> Option<Self> {
        let p0 = plist.get(0)?;
        let mut ret = AABB(*p0, *p0);
        for p in plist {
            ret.include(*p)
        }
        Some(ret)
    }

    pub fn padded(&self, padding: f32) -> Self {
        let d = Vec2::new(padding, padding);
        AABB(self.0 - d, self.1 + d)
    }

    pub fn include(&mut self, p: Vec2) {
        self.0.x = self.0.x.min(p.x);
        self.0.y = self.0.y.min(p.y);
        self.1.x = self.1.x.max(p.x);
        self.1.y = self.1.y.max(p.y);
    }

    pub fn center(&self) -> Vec2 {
        (self.0 + self.1) / 2.0
    }

    pub fn span(&self) -> Vec2 {
        self.1 - self.0
    }

    pub fn to_normalized(&self, p: Vec2) -> Vec2 {
        let u = p - self.0;
        let s = self.span();
        u / s
    }

    pub fn from_normalized(&self, u: Vec2) -> Vec2 {
        u * self.span() + self.0
    }

    pub fn map(from: Self, to: Self, p: Vec2) -> Vec2 {
        let u = from.to_normalized(p);
        to.from_normalized(u)
    }

    pub fn contains(&self, p: Vec2) -> bool {
        let u = self.to_normalized(p);
        0.0 <= u.x && u.x <= 1.0 && 0.0 <= u.y && u.y <= 1.0
    }
}
