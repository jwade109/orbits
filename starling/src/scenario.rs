use crate::nanotime::Nanotime;
use crate::orbiter::*;
use crate::orbits::{Body, SparseOrbit};
use crate::planning::EventType;
use crate::pv::PV;
use serde::{Serialize, Deserialize};
use glam::f32::Vec2;

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

#[derive(Debug, Copy, Clone)]
pub enum ScenarioObject<O, B> {
    Orbiter(O),
    Body(B),
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectLookup<O, B> {
    pub inner: ScenarioObject<O, B>,
    pub local_pv: PV,
    pub frame_pv: PV,
    pub parent: Option<ObjectId>,
}

impl<O: Copy, B: Copy> ObjectLookup<O, B> {
    pub fn pv(&self) -> PV {
        self.local_pv + self.frame_pv
    }

    pub fn orbiter(&self) -> Option<O> {
        match &self.inner {
            ScenarioObject::Orbiter(o) => Some(*o),
            _ => None,
        }
    }

    pub fn body(&self) -> Option<B> {
        match &self.inner {
            ScenarioObject::Body(b) => Some(*b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    objects: Vec<Orbiter>,
    pub system: PlanetarySystem,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

    pub fn all_ids(&self) -> Vec<ObjectId> {
        self.ids()
            .into_iter()
            .chain(self.system.ids().into_iter())
            .collect()
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

    pub fn dv(&mut self, id: ObjectId, stamp: Nanotime, dv: Vec2) -> Option<()> {
        let obj = self.objects.iter_mut().find(|o| o.id() == id)?;
        obj.dv(stamp, dv)
    }

    pub fn retain<F: FnMut(&Orbiter) -> bool>(&mut self, f: F) {
        self.objects.retain(f)
    }

    pub fn prop_count(&self) -> usize {
        self.objects.iter().map(|o| o.props().len()).sum()
    }

    pub fn orbiter_count(&self) -> usize {
        self.objects.len()
    }

    pub fn lookup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup<&Orbiter, Body>> {
        let pl = self.system.lookup(id, stamp);
        if let Some((body, pv, parent, _)) = pl {
            return Some(ObjectLookup {
                inner: ScenarioObject::Body(body),
                local_pv: PV::zero(),
                frame_pv: pv,
                parent,
            });
        }

        self.objects.iter().find_map(|o| {
            if o.id() != id {
                return None;
            }

            let prop = o.propagator_at(stamp)?;

            let (_, frame_pv, _, _) = self.system.lookup(prop.parent, stamp)?;

            Some(ObjectLookup {
                inner: ScenarioObject::Orbiter(o),
                local_pv: prop.pv(stamp)?,
                frame_pv,
                parent: Some(prop.parent),
            })
        })
    }
}
