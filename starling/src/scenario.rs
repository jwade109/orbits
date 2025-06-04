use crate::nanotime::Nanotime;
use crate::orbiter::*;
use crate::orbits::{Body, SparseOrbit};
use crate::planning::EventType;
use crate::pv::PV;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct ObjectIdTracker(OrbiterId, PlanetId);

impl ObjectIdTracker {
    pub fn new() -> Self {
        ObjectIdTracker(OrbiterId(0), PlanetId(0))
    }

    pub fn next(&mut self) -> OrbiterId {
        let ret = self.0;
        self.0 .0 += 1;
        ret
    }

    pub fn next_planet(&mut self) -> PlanetId {
        let ret = self.1;
        self.1 .0 += 1;
        ret
    }
}

#[derive(Debug, Clone)]
pub enum ScenarioObject<'a> {
    Orbiter(&'a Orbiter),
    Body(&'a String, Body),
}

#[derive(Debug, Clone)]
pub struct ObjectLookup<'a>(ObjectId, ScenarioObject<'a>, PV);

impl<'a> ObjectLookup<'a> {
    pub fn id(&self) -> ObjectId {
        self.0
    }

    pub fn pv(&self) -> PV {
        self.2
    }

    pub fn orbiter(&self) -> Option<&'a Orbiter> {
        match self.1 {
            ScenarioObject::Orbiter(o) => Some(o),
            _ => None,
        }
    }

    pub fn parent(&self, stamp: Nanotime) -> Option<PlanetId> {
        let orbiter = self.orbiter()?;
        let prop = orbiter.propagator_at(stamp)?;
        Some(prop.parent())
    }

    pub fn body(&self) -> Option<Body> {
        match self.1 {
            ScenarioObject::Body(_, b) => Some(b),
            _ => None,
        }
    }

    pub fn named_body(&self) -> Option<(&'a String, Body)> {
        match self.1 {
            ScenarioObject::Body(s, b) => Some((s, b)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlanetarySystem {
    pub id: PlanetId,
    pub name: String,
    pub body: Body,
    pub subsystems: Vec<(SparseOrbit, PlanetarySystem)>,
}

impl PlanetarySystem {
    pub fn new(id: PlanetId, name: impl Into<String>, body: Body) -> Self {
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

    pub fn planet_ids(&self) -> Vec<PlanetId> {
        let mut ret = vec![self.id];
        for (_, sub) in &self.subsystems {
            ret.extend_from_slice(&sub.planet_ids())
        }
        ret
    }

    pub fn bodies<T: Into<Option<PV>>>(
        &self,
        stamp: Nanotime,
        origin: T,
    ) -> impl Iterator<Item = (PV, Body)> + use<'_, T> {
        let origin = origin.into().unwrap_or(PV::ZERO);
        let mut ret = vec![(origin, self.body)];
        for (orbit, sys) in &self.subsystems {
            if let Ok(pv) = orbit.pv(stamp) {
                let r = sys.bodies(stamp, pv);
                ret.extend(r);
            }
        }
        ret.into_iter()
    }

    fn lookup_inner(
        &self,
        id: PlanetId,
        stamp: Nanotime,
        wrt: PV,
        parent_id: Option<PlanetId>,
    ) -> Option<(Body, PV, Option<PlanetId>, &PlanetarySystem)> {
        if self.id == id {
            return Some((self.body, wrt, parent_id, self));
        }

        for (orbit, pl) in &self.subsystems {
            if let Some(pv) = orbit.pv(stamp).ok() {
                let ret = pl.lookup_inner(id, stamp, wrt + pv, Some(self.id));
                if let Some(r) = ret {
                    return Some(r);
                }
            }
        }

        None
    }

    pub fn lookup(
        &self,
        id: PlanetId,
        stamp: Nanotime,
    ) -> Option<(Body, PV, Option<PlanetId>, &PlanetarySystem)> {
        self.lookup_inner(id, stamp, PV::ZERO, None)
    }

    pub fn potential_at(&self, pos: Vec2, stamp: Nanotime) -> f32 {
        let r = pos.length().clamp(10.0, std::f32::MAX);
        let mut ret = -self.body.mu() / r;
        for (orbit, pl) in &self.subsystems {
            if let Some(pv) = orbit.pv(stamp).ok() {
                ret += pl.potential_at(pos - pv.pos_f32(), stamp);
            }
        }
        ret
    }
}

#[derive(Debug, Clone)]
pub struct RemovalInfo {
    pub stamp: Nanotime,
    pub reason: EventType,
    pub parent: PlanetId,
    pub orbit: SparseOrbit,
}

impl RemovalInfo {
    pub fn pv(&self) -> Option<PV> {
        self.orbit.pv(self.stamp).ok()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    pub orbiters: HashMap<OrbiterId, Orbiter>,
    pub planets: PlanetarySystem,
}

impl Scenario {
    pub fn new(system: &PlanetarySystem) -> Self {
        Scenario {
            orbiters: HashMap::new(),
            planets: system.clone(),
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = ObjectId> + use<'_> {
        self.orbiter_ids().map(|id| ObjectId::Orbiter(id)).chain(
            self.planets
                .planet_ids()
                .into_iter()
                .map(|id| ObjectId::Planet(id)),
        )
    }

    pub fn orbiter_ids(&self) -> impl Iterator<Item = OrbiterId> + use<'_> {
        self.orbiters.keys().into_iter().map(|id| *id)
    }

    pub fn lup_planet(&self, id: PlanetId, stamp: Nanotime) -> Option<ObjectLookup> {
        let (body, pv, _, sys) = self.planets.lookup(id, stamp)?;
        Some(ObjectLookup(
            ObjectId::Planet(id),
            ScenarioObject::Body(&sys.name, body),
            pv,
        ))
    }

    pub fn lup_orbiter(&self, id: OrbiterId, stamp: Nanotime) -> Option<ObjectLookup> {
        let orbiter = self.orbiters.get(&id)?;

        let prop = orbiter.propagator_at(stamp)?;

        let (_, frame_pv, _, _) = self.planets.lookup(prop.parent(), stamp)?;

        let local_pv = prop.pv(stamp)?;
        let pv = frame_pv + local_pv;

        Some(ObjectLookup(
            ObjectId::Orbiter(id),
            ScenarioObject::Orbiter(orbiter),
            pv,
        ))
    }

    pub fn relevant_body(
        planets: &PlanetarySystem,
        pos: Vec2,
        stamp: Nanotime,
    ) -> Option<PlanetId> {
        let results = planets
            .planet_ids()
            .into_iter()
            .filter_map(|id| {
                let (body, pv, _, _) = planets.lookup(id, stamp)?;
                let p = pv.pos_f32();
                let d = pos.distance(p);
                (d <= body.soi).then(|| (d, id))
            })
            .collect::<Vec<_>>();
        results
            .iter()
            .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
            .map(|(_, id)| *id)
    }

    pub fn nearest(&self, pos: Vec2, stamp: Nanotime) -> Option<ObjectId> {
        let results = self
            .ids()
            .filter_map(|id| {
                let lup = match id {
                    ObjectId::Orbiter(id) => self.lup_orbiter(id, stamp),
                    ObjectId::Planet(id) => self.lup_planet(id, stamp),
                }?;
                let p = lup.pv().pos_f32();
                let d = pos.distance(p);
                Some((d, id))
            })
            .collect::<Vec<_>>();
        results
            .into_iter()
            .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
            .map(|(_, id)| id)
    }

    pub fn simulate(
        orbiters: &mut HashMap<OrbiterId, Orbiter>,
        planets: &PlanetarySystem,
        stamp: Nanotime,
        future_dur: Nanotime,
    ) -> Vec<(OrbiterId, RemovalInfo)> {
        for (_, obj) in orbiters.iter_mut() {
            let e = obj.propagate_to(stamp, future_dur, planets);
            if let Err(_e) = e {
                // dbg!(e);
            }
        }

        let mut info = vec![];

        orbiters.retain(|_, o| {
            if o.propagator_at(stamp).is_none() {
                let reason = o.props().last().map(|p| RemovalInfo {
                    stamp: p.end().unwrap_or(stamp),
                    reason: p.event().unwrap_or(EventType::NumericalError),
                    parent: p.parent(),
                    orbit: p.orbit.1,
                });
                if let Some(reason) = reason {
                    info.push((o.id(), reason));
                }
                false
            } else {
                true
            }
        });

        info
    }
}
