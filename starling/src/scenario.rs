use crate::id::*;
use crate::nanotime::Nanotime;
use crate::orbits::{Body, SparseOrbit};
use crate::pv::PV;
use crate::quantities::*;
use crate::universe::Universe;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlanetarySystem {
    id: EntityId,
    name: String,
    body: Body,
    subsystems: Vec<(SparseOrbit, Self)>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Planet {
    pub name: String,
    pub body: Body,
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

    pub fn name(&self) -> &str {
        &self.name
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

    pub fn bodies(&self) -> Vec<(EntityId, &SparseOrbit, f64)> {
        self.subsystems
            .iter()
            .map(|(orbit, pl)| (pl.id, orbit, pl.body.soi))
            .collect::<Vec<_>>()
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

// pub fn rss() -> Universe {
//     let earth_body = Body::with_mu(EARTH_RADIUS, EARTH_MU, EARTH_SOI);
//     let mut earth = PlanetarySystem::new(900, "Earth", earth_body);

//     let luna_body = Body::with_mu(LUNA_RADIUS, LUNA_MU, LUNA_SOI);
//     let mut luna = PlanetarySystem::new(901, "Luna", luna_body);
//     let luna_orbit = SparseOrbit::circular(
//         LUNA_ORBITAL_RADIUS as f64,
//         earth_body,
//         Nanotime::zero(),
//         false,
//     );

//     let ast_body = Body::with_mu(LUNA_RADIUS * 0.01, LUNA_MU * 0.0001, LUNA_RADIUS * 0.02);
//     let ast = PlanetarySystem::new(902, "Asteroid", ast_body);
//     let ast_orbit =
//         SparseOrbit::circular(LUNA_RADIUS * 1.5 as f64, luna_body, Nanotime::zero(), true);

//     luna.subsystems.push((ast_orbit, ast));
//     earth.subsystems.push((luna_orbit, luna));

//     Universe::new(earth)
// }

pub fn rss() -> Universe {
    Universe::empty()
}
