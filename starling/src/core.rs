use crate::orbit::*;
use bevy::math::Vec2;
use rand::Rng;
use std::ops::Add;
use chrono::TimeDelta;

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

pub fn gravity_accel(body: Body, body_center: Vec2, sample: Vec2) -> Vec2 {
    let r: Vec2 = body_center - sample;
    let rsq = r.length_squared().clamp(body.radius.powi(2), std::f32::MAX);
    let a = GRAVITATIONAL_CONSTANT * body.mass / rsq;
    a * r.normalize()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId(pub i64);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventId(pub i64);

#[derive(Debug, Clone)]
pub struct OrbitalSystem {
    pub primary: Body,
    pub epoch: TimeDelta,
    pub objects: Vec<(ObjectId, Orbit)>,
    next_id: i64,
    pub subsystems: Vec<(ObjectId, Orbit, OrbitalSystem)>,
    metadata: Vec<ObjectMetadata>,
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

#[derive(Debug, Copy, Clone)]
pub enum ObjectType {
    Orbiter,
    System,
}

#[derive(Debug, Copy, Clone)]
pub enum OrbitStability {
    Unknown,
    Perpetual,
    OnEscape(Option<TimeDelta>),
    SubOrbital(Option<TimeDelta>),
    MightEncounter(Option<TimeDelta>),
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectMetadata {
    pub id: ObjectId,
    pub stability: OrbitStability,
}

impl OrbitalSystem {
    pub fn new(body: Body) -> Self {
        OrbitalSystem {
            primary: body,
            epoch: TimeDelta::default(),
            objects: Vec::default(),
            next_id: 0,
            subsystems: vec![],
            metadata: vec![],
        }
    }

    pub fn add_object(&mut self, orbit: Orbit) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.objects.push((id, orbit));
        self.calculate_metadata();
        id
    }

    pub fn add_subsystem(&mut self, orbit: Orbit, subsys: OrbitalSystem) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        self.subsystems.push((id, orbit, subsys));
        self.calculate_metadata();
        id
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

    pub fn lookup_metadata(&self, o: ObjectId) -> Option<&ObjectMetadata> {
        self.metadata.iter().find(|dat| dat.id == o)
    }

    pub fn transform_from_id(&self, id: ObjectId, stamp: TimeDelta) -> Option<PV> {
        let orbit = self.lookup(id)?;
        Some(orbit.pv_at_time(stamp))
    }

    pub fn potential_at(&self, pos: Vec2, stamp: TimeDelta) -> f32 {
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

    pub fn calculate_metadata(&mut self) {
        self.metadata.clear();

        for (id, orbit) in &self.objects {
            let might_encounter = self
                .subsystems
                .iter()
                .any(|(_, sysorb, sys)| can_intersect_soi(orbit, sysorb, sys.primary.soi));

            let stability = if might_encounter {
                OrbitStability::MightEncounter(None)
            } else if will_hit_body(orbit, self.primary.radius) {
                OrbitStability::SubOrbital(None)
            } else {
                OrbitStability::Perpetual
            };
            self.metadata.push(ObjectMetadata { id: *id, stability });
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

pub fn synodic_period(t1: f32, t2: f32) -> Option<f32> {
    if t1 == t2 {
        None
    } else {
        Some(t1 * t2 / (t2 - t1).abs())
    }
}
