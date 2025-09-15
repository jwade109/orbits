use crate::id::*;
use crate::nanotime::Nanotime;
use crate::orbits::{Body, SparseOrbit};
use crate::pv::PV;
use serde::{Deserialize, Serialize};

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

    pub fn planet_ids(&self) -> Vec<EntityId> {
        let mut ret = vec![self.id];
        for (_, sub) in &self.subsystems {
            ret.extend_from_slice(&sub.planet_ids())
        }
        ret
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
                if let Some((body, pv, parent, sys)) = ret {
                    return Some((body, pv, parent, sys));
                }
            }
        }

        None
    }

    pub fn lookup_planet(
        &self,
        id: EntityId,
        stamp: Nanotime,
    ) -> Option<(Body, PV, Option<EntityId>, &Self)> {
        self.lookup_inner(id, stamp, PV::ZERO, None)
    }
}
