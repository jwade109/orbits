use crate::starling::math::*;
use crate::starling::orbits::{Body, SparseOrbit};
use bevy::prelude::Entity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum PlanetarySystem {
    Void,
    Planet {
        id: Entity,
        name: String,
        body: Body,
        subsystems: Vec<(SparseOrbit, Self)>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Planet {
    pub name: String,
    pub body: Body,
}

impl PlanetarySystem {
    pub fn new(id: Entity, name: impl Into<String>, body: Body) -> Self {
        Self::Planet {
            id,
            name: name.into(),
            body,
            subsystems: vec![],
        }
    }

    pub fn planet_ids(&self) -> Vec<Entity> {
        match self {
            Self::Void => vec![],
            Self::Planet {
                id,
                name: _,
                body: _,
                subsystems,
            } => {
                let mut ret = vec![*id];
                for (_, sub) in subsystems {
                    ret.extend_from_slice(&sub.planet_ids())
                }
                ret
            }
        }
    }

    fn lookup_inner(
        &self,
        lup_id: Entity,
        stamp: Nanotime,
        wrt: PV,
        parent_id: Option<Entity>,
    ) -> Option<(Planet, PV, Option<Entity>, &Self)> {
        match self {
            Self::Void => None,
            Self::Planet {
                id,
                name,
                body,
                subsystems,
            } => {
                if lup_id == *id {
                    let planet = Planet {
                        name: name.clone(),
                        body: *body,
                    };
                    return Some((planet, wrt, parent_id, self));
                }

                for (orbit, pl) in subsystems {
                    if let Some(pv) = orbit.pv(stamp).ok() {
                        let ret = pl.lookup_inner(lup_id, stamp, wrt + pv, Some(*id));
                        if let Some((body, pv, parent, sys)) = ret {
                            return Some((body, pv, parent, sys));
                        }
                    }
                }

                None
            }
        }
    }

    pub fn lookup_planet(
        &self,
        id: Entity,
        stamp: Nanotime,
    ) -> Option<(Planet, PV, Option<Entity>, &Self)> {
        self.lookup_inner(id, stamp, PV::ZERO, None)
    }
}

pub fn make_earth() -> Body {
    Body::with_mass(63.0, 1000.0, 15000.0)
}

pub fn make_luna() -> (Body, SparseOrbit) {
    (
        Body::with_mass(22.0, 10.0, 800.0),
        SparseOrbit::circular(3800.0, make_earth(), Nanotime::secs(-40), false),
    )
}
