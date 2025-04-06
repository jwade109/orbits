use crate::belts::AsteroidBelt;
use crate::math::{rand, rotate, vproj, PI};
use crate::nanotime::Nanotime;
use crate::orbiter::*;
use crate::orbits::{Body, GlobalOrbit, SparseOrbit};
use crate::planning::EventType;
use crate::pv::PV;
use glam::f32::Vec2;
use serde::{Deserialize, Serialize};

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

    pub fn parent(&self, stamp: Nanotime) -> Option<ObjectId> {
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
        id: ObjectId,
        stamp: Nanotime,
    ) -> Option<(Body, PV, Option<ObjectId>, &PlanetarySystem)> {
        self.lookup_inner(id, stamp, PV::zero(), None)
    }

    pub fn potential_at(&self, pos: Vec2, stamp: Nanotime) -> f32 {
        let r = pos.length().clamp(10.0, std::f32::MAX);
        let mut ret = -self.body.mu() / r;
        for (orbit, pl) in &self.subsystems {
            if let Some(pv) = orbit.pv(stamp).ok() {
                ret += pl.potential_at(pos - pv.pos, stamp);
            }
        }
        ret
    }
}

#[derive(Debug, Clone)]
pub struct RemovalInfo {
    pub stamp: Nanotime,
    pub reason: EventType,
    pub parent: ObjectId,
    pub orbit: SparseOrbit,
}

impl RemovalInfo {
    pub fn pv(&self) -> Option<PV> {
        self.orbit.pv(self.stamp).ok()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scenario {
    orbiters: Vec<Orbiter>,
    system: PlanetarySystem,
    debris: Vec<GlobalOrbit>,
    belts: Vec<AsteroidBelt>,
}

impl Scenario {
    pub fn new(system: &PlanetarySystem) -> Self {
        Scenario {
            orbiters: vec![],
            system: system.clone(),
            belts: vec![],
            debris: vec![],
        }
    }

    pub fn orbiter_ids(&self) -> impl Iterator<Item = ObjectId> + use<'_> {
        self.orbiters.iter().map(|o| o.id())
    }

    pub fn planet_ids(&self) -> Vec<ObjectId> {
        self.system.ids()
    }

    pub fn all_ids(&self) -> Vec<ObjectId> {
        self.orbiter_ids()
            .into_iter()
            .chain(self.system.ids().into_iter())
            .collect()
    }

    pub fn has_orbiter(&self, id: ObjectId) -> bool {
        self.orbiters.iter().any(|o| o.id() == id)
    }

    pub fn planets(&self) -> &PlanetarySystem {
        &self.system
    }

    pub fn belts(&self) -> &Vec<AsteroidBelt> {
        &self.belts
    }

    pub fn debris(&self) -> impl Iterator<Item = &GlobalOrbit> + use<'_> {
        self.debris.iter()
    }

    pub fn simulate(
        &mut self,
        stamp: Nanotime,
        future_dur: Nanotime,
    ) -> Vec<(ObjectId, RemovalInfo)> {
        for obj in &mut self.orbiters {
            let e = obj.propagate_to(stamp, future_dur, &self.system);
            if let Err(_e) = e {
                // dbg!(e);
            }
        }

        let mut info = vec![];

        self.orbiters.retain(|o| {
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

        for (_, info) in &info {
            let pv = match info.pv() {
                Some(pv) => pv,
                None => continue,
            };

            for _ in 0..20 {
                let pos = pv.pos;
                let vmag = pv.vel.length();
                let (v_normal, v_tangent) = vproj(pv.vel, pos);

                let n = pos.normalize_or_zero();
                let t = rotate(n, PI / 2.0);

                let v_normal = -v_normal * rand(0.01, 0.1) + n * rand(0.03, 0.1) * vmag;
                let v_tangent = v_tangent * rand(0.01, 0.1) + t * rand(-1.0, 1.0) * vmag * 0.1;

                let mut body = info.orbit.body;
                body.mass *= 0.5;

                let pv = PV::new(pos, v_normal + v_tangent);
                if let Some(orbit) = SparseOrbit::from_pv(pv, body, info.stamp) {
                    self.debris.push(GlobalOrbit(info.parent, orbit));
                }
            }
        }

        self.debris.retain(|GlobalOrbit(_, orbit)| {
            let dt = stamp - orbit.epoch;
            if dt > Nanotime::secs(5) {
                return false;
            }
            let pv = match orbit.pv(stamp).ok() {
                Some(pv) => pv,
                None => return false,
            };
            let r = pv.pos.length();
            r > orbit.body.radius && r < orbit.body.soi
        });

        info
    }

    pub fn add_belt(&mut self, belt: AsteroidBelt) {
        self.belts.push(belt);
    }

    pub fn add_object(
        &mut self,
        id: ObjectId,
        parent: ObjectId,
        orbit: SparseOrbit,
        stamp: Nanotime,
    ) {
        self.orbiters
            .push(Orbiter::new(id, GlobalOrbit(parent, orbit), stamp));
    }

    pub fn remove_object(&mut self, id: ObjectId) -> Option<()> {
        self.orbiters
            .remove(self.orbiters.iter().position(|o| o.id() == id)?);
        Some(())
    }

    pub fn impulsive_burn(&mut self, id: ObjectId, stamp: Nanotime, dv: Vec2) -> Option<()> {
        let obj = self.orbiters.iter_mut().find(|o| o.id() == id)?;
        obj.impulsive_burn(stamp, dv)?;

        Some(())
    }

    // TODO get rid of this
    pub fn retain<F: FnMut(&Orbiter) -> bool>(&mut self, f: F) {
        self.orbiters.retain(f)
    }

    pub fn prop_count(&self) -> usize {
        self.orbiters.iter().map(|o| o.props().len()).sum()
    }

    pub fn orbiter_count(&self) -> usize {
        self.orbiters.len()
    }

    pub fn lup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup> {
        let pl = self.system.lookup(id, stamp);
        if let Some((body, pv, _, sys)) = pl {
            return Some(ObjectLookup(id, ScenarioObject::Body(&sys.name, body), pv));
        }

        self.orbiters.iter().find_map(|o| {
            if o.id() != id {
                return None;
            }

            let prop = o.propagator_at(stamp)?;

            let (_, frame_pv, _, _) = self.system.lookup(prop.parent(), stamp)?;

            let local_pv = prop.pv(stamp)?;
            let pv = frame_pv + local_pv;

            Some(ObjectLookup(id, ScenarioObject::Orbiter(o), pv))
        })
    }

    pub fn relevant_body(&self, pos: Vec2, stamp: Nanotime) -> Option<ObjectId> {
        let results = self
            .system
            .ids()
            .into_iter()
            .filter_map(|id| {
                let lup = self.lup(id, stamp)?;
                let p = lup.pv().pos;
                let body = lup.body()?;
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
            .all_ids()
            .into_iter()
            .filter_map(|id| {
                let lup = self.lup(id, stamp)?;
                let p = lup.pv().pos;
                let d = pos.distance(p);
                Some((d, id))
            })
            .collect::<Vec<_>>();
        results
            .into_iter()
            .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
            .map(|(_, id)| id)
    }
}
