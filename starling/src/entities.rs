use crate::prelude::*;

#[derive(Debug)]
pub struct OrbitalSpacecraftEntity {
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub orbiter: Orbiter,
    pub controller: OrbitalController,
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
        }
    }
}

#[derive(Debug)]
pub struct SurfaceSpacecraftEntity {
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub controller: VehicleController,
}

impl SurfaceSpacecraftEntity {
    pub fn new(vehicle: Vehicle, body: RigidBody, controller: VehicleController) -> Self {
        Self {
            vehicle,
            body,
            controller,
        }
    }
}
