use crate::id::*;
use crate::nanotime::Nanotime;
use crate::orbits::{Body, SparseOrbit, GlobalOrbit};
use crate::pv::PV;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ObjectLookup<'a>(&'a String, Body);

impl<'a> ObjectLookup<'a> {
    pub fn new<'b: 'a>(stuff: (&'b String, Body)) -> Self {
        Self(stuff.0, stuff.1)
    }

    pub fn named_body(&self) -> (&'a String, Body) {
        (self.0, self.1)
    }
}

pub struct PlanetaryBody {
    name: String,
    body: Body,
    orbit: Option<GlobalOrbit>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlanetarySystem {
    pub id: EntityId,
    pub name: String,
    pub body: Body,
    pub subsystems: Vec<(SparseOrbit, Self)>,
}

impl PlanetarySystem {
    pub fn new(id: i64, name: impl Into<String>, body: Body) -> Self {
        Self {
            id: EntityId(id),
            name: name.into(),
            body,
            subsystems: vec![],
        }
    }

    pub fn orbit(&mut self, orbit: SparseOrbit, planets: Self) {
        self.subsystems.push((orbit, planets));
    }

    pub fn planet_ids(&self) -> Vec<EntityId> {
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
        id: EntityId,
        stamp: Nanotime,
        wrt: PV,
        parent_id: Option<EntityId>,
    ) -> Option<(Body, PV, Option<EntityId>, &Self)> {
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

    #[deprecated]
    pub fn lookup(
        &self,
        id: EntityId,
        stamp: Nanotime,
    ) -> Option<(Body, PV, Option<EntityId>, &Self)> {
        self.lookup_inner(id, stamp, PV::ZERO, None)
    }
}
