use crate::nanotime::Nanotime;
use crate::orbiter::*;
use crate::orbits::{Body, SparseOrbit};
use crate::planning::EventType;
use crate::pv::PV;
use glam::f32::Vec2;

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
pub struct Scenario {
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

    pub fn ids(&self) -> Vec<ObjectId> {
        let mut ret = vec![self.id];
        for (_, sub) in &self.subsystems {
            ret.extend_from_slice(&sub.ids())
        }
        ret
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

    pub fn potential_at(&self, pos: Vec2, stamp: Nanotime) -> f32 {
        let r = pos.length().clamp(10.0, std::f32::MAX);
        let mut ret = -self.body.mu() / r;
        for (orbit, pl) in &self.subsystems {
            let pv = orbit.pv_at_time(stamp);
            ret += pl.potential_at(pos - pv.pos, stamp);
        }
        ret
    }
}

#[derive(Debug, Clone)]
pub struct RemovalInfo {
    pub stamp: Nanotime,
    pub reason: EventType,
    pub orbit: SparseOrbit,
}

impl Scenario {
    pub fn new(system: &PlanetarySystem) -> Self {
        Scenario {
            objects: vec![],
            system: system.clone(),
        }
    }

    pub fn ids(&self) -> Vec<ObjectId> {
        self.objects.iter().map(|o| o.id()).collect()
    }

    pub fn simulate(
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
                info.push((o.id(), reason));
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
            .remove(self.objects.iter().position(|o| o.id() == id)?);
        Some(())
    }

    pub fn orbiter_lookup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup> {
        self.objects.iter().find_map(|o| {
            if o.id() != id {
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
