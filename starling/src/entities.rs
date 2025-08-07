use crate::prelude::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct OrbitalSpacecraftEntity {
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub orbiter: Orbiter,
    pub controller: OrbitalController,
    pub reference_orbit_age: Nanotime,
}

impl OrbitalSpacecraftEntity {
    pub fn new(
        vehicle: Vehicle,
        body: RigidBody,
        orbiter: Orbiter,
        controller: OrbitalController,
    ) -> Self {
        Self {
            vehicle,
            body,
            orbiter,
            controller,
            reference_orbit_age: Nanotime::ZERO,
        }
    }

    pub fn current_orbit(&self, stamp: Nanotime) -> Option<GlobalOrbit> {
        let body_pv = self.body.pv; // m/s
        let GlobalOrbit(id, orbit) = self.orbiter.orbit(stamp)?;
        if body_pv.is_zero() {
            return Some(GlobalOrbit(*id, *orbit));
        }
        let orbit_pv = orbit.pv(stamp).ok()?; // km/s
        let pv = orbit_pv + body_pv / 1000.0;
        let orbit = SparseOrbit::from_pv(pv, orbit.body, stamp)?;
        Some(GlobalOrbit(*id, orbit))
    }
}

#[derive(Debug)]
pub struct SurfaceSpacecraftEntity {
    pub surface_id: EntityId,
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub controller: VehicleController,
    pub orbit: Option<SparseOrbit>,
}

impl SurfaceSpacecraftEntity {
    pub fn new(
        surface_id: EntityId,
        vehicle: Vehicle,
        body: RigidBody,
        controller: VehicleController,
    ) -> Self {
        Self {
            surface_id,
            vehicle,
            body,
            controller,
            orbit: None,
        }
    }
}

#[derive(Debug)]
pub struct LandingSiteEntity {
    pub name: String,
    pub surface: Surface,
    pub planet: EntityId,
    pub angle: f64,
    pub tracks: HashMap<EntityId, Vec<(Nanotime, DVec2)>>,
}

impl LandingSiteEntity {
    pub fn new(name: String, surface: Surface, planet: EntityId, angle: f64) -> Self {
        Self {
            name,
            surface,
            planet,
            angle,
            tracks: HashMap::new(),
        }
    }

    pub fn add_position_track(&mut self, id: EntityId, stamp: Nanotime, p: DVec2) {
        if let Some(track) = self.tracks.get_mut(&id) {
            if let Some((t, _)) = track.last() {
                let dt = stamp - *t;
                if dt > Nanotime::secs(1) {
                    track.push((stamp, p));
                }
            } else {
                track.push((stamp, p));
            }

            if track.len() > 120 {
                track.remove(0);
            }
        } else {
            let track = vec![(stamp, p)];
            self.tracks.insert(id, track);
        }
    }
}

pub fn landing_site_info(ls: &LandingSiteEntity) -> String {
    [
        format!("{}", ls.name),
        format!("Planet: {}", ls.planet),
        format!("Atmo color: {:?}", ls.surface.atmo_color),
    ]
    .into_iter()
    .map(|s| format!("{s}\n"))
    .collect()
}
