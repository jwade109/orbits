use crate::orbit::*;
use crate::planning::*;
use crate::pv::PV;
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
    pub parent: ObjectId,
    pub orbit: Orbit,
    pub propagator: Propagator,
}

impl Object {
    pub fn new(id: ObjectId, parent: ObjectId, orbit: Orbit, stamp: Nanotime) -> Self {
        Object {
            id,
            parent,
            orbit,
            propagator: Propagator::new(stamp),
        }
    }

    pub fn next(&self, planet: &Planet, event: OrbitalEvent) -> Option<Object> {
        let mut o = *self;
        match event.etype {
            EventType::Maneuver(dv) => {
                let (body, _) = planet.lookup(self.parent, event.stamp)?;
                let pv = self.orbit.pv_at_time(event.stamp);
                let new_pv = pv + PV::vel(dv);
                let orbit = Orbit::from_pv(new_pv.pos, new_pv.vel, body.mass, event.stamp);
                o.orbit = orbit;
            }
            _ => (),
        }
        Some(o)
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

impl PartialEq for EventType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EventType::Collide, EventType::Collide) => true,
            (EventType::Escape, EventType::Escape) => true,
            (EventType::Encounter(o), EventType::Encounter(p)) => o == p,
            _ => false,
        }
    }
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

#[derive(Debug, Clone)]
pub struct ObjectLookup {
    pub object: Object,
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
    pub objects: Vec<Object>,
    pub events: Vec<OrbitalEvent>,
    pub system: Planet,
}

#[derive(Debug, Clone)]
pub struct Planet {
    pub id: ObjectId,
    pub primary: Body,
    pub subsystems: Vec<(Orbit, Planet)>,
}

impl Planet {
    pub fn new(id: ObjectId, primary: Body) -> Self {
        Planet {
            id,
            primary,
            subsystems: vec![],
        }
    }

    pub fn orbit(&mut self, orbit: Orbit, planet: Planet) {
        self.subsystems.push((orbit, planet));
    }

    fn lookup_inner(&self, id: ObjectId, stamp: Nanotime, wrt: PV) -> Option<(Body, PV)> {
        if self.id == id {
            return Some((self.primary, wrt));
        }

        for (orbit, pl) in &self.subsystems {
            let pv = orbit.pv_at_time(stamp);
            let ret = pl.lookup_inner(id, stamp, wrt + pv);
            if let Some(r) = ret {
                return Some(r);
            }
        }

        None
    }

    pub fn lookup(&self, id: ObjectId, stamp: Nanotime) -> Option<(Body, PV)> {
        self.lookup_inner(id, stamp, PV::zero())
    }
}

impl OrbitalTree {
    pub fn new(system: &Planet) -> Self {
        OrbitalTree {
            objects: vec![],
            events: vec![],
            system: system.clone(),
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
                // let l1 = self.lookup(pri, event.stamp)?;
                // let b = l1.body?;
                // let l2 = self.lookup(event.target, event.stamp)?;
                // let d = l2.pv() - l1.pv();
                // let orbit = Orbit::from_pv(d.pos, d.vel, b.mass, event.stamp);
                // self.remove_object(event.target);
                // let (_, sys) = self.subsystems.iter_mut().find(|(obj, _)| obj.id == pri)?;
                // sys.add_object(event.target, orbit, event.stamp)
            }
            EventType::Maneuver(dv) => {
                // let lup = self.lookup(event.target, event.stamp)?;
                // let dpv = PV::new(Vec2::ZERO, dv);
                // let pv = lup.pv() + dpv;
                // let orbit = Orbit::from_pv(pv.pos, pv.vel, self.system.primary.mass, event.stamp);
                // self.remove_object(event.target);
                // self.add_object(event.target, orbit, event.stamp);
            }
        };
        None
    }

    pub fn propagate_to(&mut self, stamp: Nanotime) {
        // let mut orbits_to_add = vec![];

        // for (orb, subsys) in &mut self.subsystems {
        //     let lups = subsys.propagate_to(stamp);
        //     let pvsys = orb.pv_at_time(stamp);
        //     for lup in lups {
        //         let pv = pvsys + lup.pv();
        //         let orbit = Orbit::from_pv(pv.pos, pv.vel, self.primary.mass, stamp);
        //         orbits_to_add.push((lup.object.id, orbit));
        //     }
        // }

        // for (id, orbit) in orbits_to_add {
        //     self.add_object(id, orbit, stamp)
        // }

        // let mut to_apply = vec![];
        // for obj in &mut self.objects {
        //     let e = obj.take_event(stamp);
        //     e.map(|e| to_apply.push(e));
        // }

        // let mut ret = vec![];
        // for e in to_apply {
        //     let lup = self.apply(e);
        //     if let Some(lup) = lup {
        //         ret.push(lup);
        //     }
        // }

        self.events.retain(|e| e.stamp > stamp);

        let bodies = self
            .system
            .subsystems
            .iter()
            .map(|(orbit, pl)| (pl.id, *orbit, pl.primary.soi))
            .collect::<Vec<_>>();

        for obj in &mut self.objects {
            while !obj.propagator.calculated_to(stamp + Nanotime::secs(200)) {
                let res = obj.propagator.next(
                    &obj.orbit,
                    self.system.primary.radius,
                    self.system.primary.soi,
                    &bodies,
                );
                match res {
                    Err(e) => {
                        dbg!(e);
                    }
                    Ok(Some((t, e))) => {
                        let event = OrbitalEvent::new(obj.id, t, e);
                        self.events.push(event);
                    }
                    _ => (),
                }
            }
        }

        self.events.sort_by_key(|e| e.stamp);

        // ret
    }

    pub fn add_object(&mut self, id: ObjectId, parent: ObjectId, orbit: Orbit, stamp: Nanotime) {
        self.objects.push(Object::new(id, parent, orbit, stamp));
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<()> {
        self.objects
            .remove(self.objects.iter().position(|o| o.id == id)?);
        Some(())
    }

    pub fn lookup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup> {
        self.objects.iter().find_map(|o| {
            if o.id != id {
                return None;
            }

            let (_, frame_pv) = self.system.lookup(o.parent, stamp)?;

            Some(ObjectLookup {
                object: *o,
                level: 0,
                local_pv: o.orbit.pv_at_time(stamp),
                frame_pv,
                otype: ObjectType::Orbiter,
                parent: ObjectId(0),
                body: None,
            })
        })
    }
}

pub fn potential_at(planet: &Planet, pos: Vec2, stamp: Nanotime) -> f32 {
    let r = pos.length().clamp(10.0, std::f32::MAX);
    let mut ret = -(planet.primary.mass * GRAVITATIONAL_CONSTANT) / r;
    for (orbit, pl) in &planet.subsystems {
        let pv = orbit.pv_at_time(stamp);
        ret += potential_at(pl, pos - pv.pos, stamp);
    }
    ret
}
