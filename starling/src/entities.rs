use crate::prelude::*;

#[derive(Debug)]
pub enum OrbitalParent {
    Planet(Orbiter),
    Vehicle(EntityId),
    Surface(EntityId),
}

impl OrbitalParent {
    pub fn orbit(&self, stamp: Nanotime) -> Option<&GlobalOrbit> {
        match self {
            Self::Planet(orbiter) => orbiter.orbit(stamp),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct OrbitalSpacecraftEntity {
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub transform: OrbitalParent,
    pub controller: OrbitalController,
    pub reference_orbit_age: Nanotime,
}

impl OrbitalSpacecraftEntity {
    pub fn new(
        vehicle: Vehicle,
        body: RigidBody,
        transform: OrbitalParent,
        controller: OrbitalController,
    ) -> Self {
        Self {
            vehicle,
            body,
            transform,
            controller,
            reference_orbit_age: Nanotime::ZERO,
        }
    }

    pub fn is_transform_orbital(&self) -> bool {
        match &self.transform {
            OrbitalParent::Planet(..) => true,
            _ => false,
        }
    }

    pub fn current_orbit(&self, stamp: Nanotime) -> Option<GlobalOrbit> {
        let body_pv = self.body.pv; // m/s
        let GlobalOrbit(id, orbit) = self.transform.orbit(stamp)?;
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
        }
    }
}

#[derive(Debug)]
pub struct LandingSiteEntity {
    pub name: String,
    pub surface: Surface,
    pub planet: EntityId,
    pub angle: f32,
    pub is_awake: bool,
}

impl LandingSiteEntity {
    pub fn new(name: String, surface: Surface, planet: EntityId, angle: f32) -> Self {
        Self {
            name,
            surface,
            planet,
            angle,
            is_awake: false,
        }
    }
}

pub fn landing_site_info(ls: &LandingSiteEntity) -> String {
    [
        format!("{}", ls.name),
        format!("Planet: {}", ls.planet),
        format!("Atmo color: {:?}", ls.surface.atmo_color),
        format!("Gravity: {}", ls.surface.gravity),
        format!("Wind: {}", ls.surface.wind),
        format!("Awake: {}", ls.is_awake),
    ]
    .into_iter()
    .map(|s| format!("{s}\n"))
    .collect()
}
