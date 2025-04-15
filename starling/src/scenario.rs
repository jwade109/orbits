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
pub struct PlanetarySystem(Vec<Planet>);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Planet {
    pub id: PlanetId,
    pub name: String,
    pub body: Body,
    pub orbit: Option<GlobalOrbit>,
}

impl Planet {
    pub fn new(
        id: PlanetId,
        name: impl Into<String>,
        body: Body,
        orbit: impl Into<Option<GlobalOrbit>>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            body,
            orbit: orbit.into(),
        }
    }

    pub fn pv(&self, stamp: Nanotime) -> Option<PV> {
        match self.orbit {
            Some(go) => go.1.pv(stamp).ok(),
            None => Some(PV::zero()),
        }
    }
}

impl PlanetarySystem {
    pub fn new() -> Self {
        PlanetarySystem(vec![])
    }

    pub fn with_body(mut self, planet: Planet) -> Self {
        self.0.push(planet);
        self
    }

    pub fn ids(&self) -> impl Iterator<Item = PlanetId> + use<'_> {
        self.0.iter().map(|p| p.id)
    }

    pub fn bodies(&self, stamp: Nanotime) -> impl Iterator<Item = (PV, Body)> + use<'_> {
        self.0.iter().filter_map(move |p| {
            let pv = if let Some(go) = p.orbit {
                go.1.pv(stamp).ok()?
            } else {
                PV::zero()
            };
            Some((pv, p.body))
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Planet> + use<'_> {
        self.0.iter()
    }

    pub fn lookup(&self, id: PlanetId, stamp: Nanotime) -> Option<(&Planet, PV)> {
        self.0
            .iter()
            .find(|p| p.id == id)
            .map(|p| {
                let pv = if let Some(go) = p.orbit {
                    go.1.pv(stamp).ok()?
                } else {
                    PV::zero()
                };
                Some((p, pv))
            })
            .flatten()
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

    pub fn ids(&self) -> impl Iterator<Item = ObjectId> + use<'_> {
        self.orbiter_ids()
            .map(|id| ObjectId::Orbiter(id))
            .chain(self.planet_ids().into_iter().map(|id| ObjectId::Planet(id)))
    }

    pub fn orbiter_ids(&self) -> impl Iterator<Item = OrbiterId> + use<'_> {
        self.orbiters.iter().map(|o| o.id())
    }

    pub fn planet_ids(&self) -> impl Iterator<Item = PlanetId> + use<'_> {
        self.system.ids()
    }

    pub fn has_orbiter(&self, id: OrbiterId) -> bool {
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

    pub fn orbiters_mut(&mut self) -> impl Iterator<Item = &mut Orbiter> + use<'_> {
        self.orbiters.iter_mut()
    }

    pub fn orbiter_mut(&mut self, id: OrbiterId) -> Option<&mut Orbiter> {
        self.orbiters.iter_mut().find(|o| o.id() == id)
    }

    pub fn simulate(
        &mut self,
        stamp: Nanotime,
        future_dur: Nanotime,
    ) -> Vec<(OrbiterId, RemovalInfo)> {
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
        id: OrbiterId,
        parent: PlanetId,
        orbit: SparseOrbit,
        stamp: Nanotime,
    ) {
        self.orbiters
            .push(Orbiter::new(id, GlobalOrbit(parent, orbit), stamp));
    }

    pub fn remove_object(&mut self, id: OrbiterId) -> Option<()> {
        self.orbiters
            .remove(self.orbiters.iter().position(|o| o.id() == id)?);
        Some(())
    }

    pub fn impulsive_burn(&mut self, id: OrbiterId, stamp: Nanotime, dv: Vec2) -> Option<()> {
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

    pub fn lup_planet(&self, id: PlanetId, stamp: Nanotime) -> Option<ObjectLookup> {
        let (pl, pv) = self.system.lookup(id, stamp)?;
        Some(ObjectLookup(
            ObjectId::Planet(id),
            ScenarioObject::Body(&pl.name, pl.body),
            pv,
        ))
    }

    pub fn lup_orbiter(&self, id: OrbiterId, stamp: Nanotime) -> Option<ObjectLookup> {
        self.orbiters.iter().find_map(|o| {
            if o.id() != id {
                return None;
            }

            let prop = o.propagator_at(stamp)?;

            let (_, frame_pv) = self.system.lookup(prop.parent(), stamp)?;

            let local_pv = prop.pv(stamp)?;
            let pv = frame_pv + local_pv;

            Some(ObjectLookup(
                ObjectId::Orbiter(id),
                ScenarioObject::Orbiter(o),
                pv,
            ))
        })
    }

    pub fn lup(&self, id: ObjectId, stamp: Nanotime) -> Option<ObjectLookup> {
        match id {
            ObjectId::Orbiter(id) => self.lup_orbiter(id, stamp),
            ObjectId::Planet(id) => self.lup_planet(id, stamp),
        }
    }

    pub fn relevant_body(&self, pos: Vec2, stamp: Nanotime) -> Option<PlanetId> {
        let results = self
            .system
            .ids()
            .into_iter()
            .filter_map(|id| {
                let lup = self.lup_planet(id, stamp)?;
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
            .ids()
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
