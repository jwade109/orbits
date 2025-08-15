use crate::prelude::*;
use std::collections::HashMap;

// #[derive(Debug)]
// pub struct OrbitalSpacecraftEntity {
//     parent_id: EntityId,
//     vehicle: Vehicle,
//     body: RigidBody,
//     orbit: Option<SparseOrbit>,
//     reference_orbit_age: Nanotime,
// }

#[derive(Debug)]
pub struct SurfaceSpacecraftEntity {
    pub planet_id: EntityId,
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub controller: VehicleController,
    pub orbit: Option<SparseOrbit>,
    pub reference_orbit_age: Nanotime,
    target: Option<EntityId>,
}

// impl OrbitalSpacecraftEntity {
//     pub fn new(parent_id: EntityId, vehicle: Vehicle, body: RigidBody) -> Self {
//         Self {
//             parent_id,
//             vehicle,
//             body,
//             orbit: None,
//             reference_orbit_age: Nanotime::ZERO,
//         }
//     }

//     pub fn pv(&self) -> PV {
//         self.body.pv
//     }

//     pub fn parent(&self) -> EntityId {
//         self.parent_id
//     }

//     pub fn current_orbit(&self) -> Option<GlobalOrbit> {
//         let orbit = self.orbit?;
//         Some(GlobalOrbit(self.parent_id, orbit))
//     }

//     pub fn vehicle(&self) -> &Vehicle {
//         &self.vehicle
//     }

//     pub fn overwrite_vehicle(&mut self, vehicle: Vehicle) {
//         self.vehicle = vehicle;
//     }

//     pub fn body(&self) -> &RigidBody {
//         &self.body
//     }

//     pub fn on_sim_tick(
//         &mut self,
//         ctrl: &VehicleControl,
//         stamp: Nanotime,
//         gravity: DVec2,
//         parent_body: Body,
//     ) {
//         self.reference_orbit_age += PHYSICS_CONSTANT_DELTA_TIME;

//         if self.reference_orbit_age > Nanotime::millis(100) {
//             self.orbit = SparseOrbit::from_pv(self.body.pv, parent_body, stamp);
//         }

//         self.vehicle.set_thrust_control(ctrl);
//         // ov.vehicle.on_sim_tick();

//         let accel = self.vehicle.body_frame_accel();

//         self.body
//             .on_sim_tick(accel, gravity, PHYSICS_CONSTANT_DELTA_TIME);
//     }

//     pub fn on_sim_tick_batch(&mut self, elapsed: Nanotime) {
//         self.body.angle += self.body.angular_velocity * elapsed.to_secs_f64();
//         self.body.angle = wrap_pi_npi_f64(self.body.angle);
//         self.vehicle.zero_all_thrusters();
//     }
// }

impl SurfaceSpacecraftEntity {
    pub fn new(
        planet_id: EntityId,
        vehicle: Vehicle,
        body: RigidBody,
        controller: VehicleController,
    ) -> Self {
        Self {
            planet_id,
            vehicle,
            body,
            controller,
            orbit: None,
            reference_orbit_age: Nanotime::ZERO,
            target: None,
        }
    }

    pub fn current_orbit(&self) -> Option<GlobalOrbit> {
        Some(GlobalOrbit(self.planet_id, self.orbit?))
    }

    pub fn vehicle(&self) -> &Vehicle {
        &self.vehicle
    }

    pub fn overwrite_vehicle(&mut self, vehicle: Vehicle) {
        self.vehicle = vehicle;
    }

    pub fn parent(&self) -> EntityId {
        self.planet_id
    }

    pub fn pv(&self) -> PV {
        self.body.pv
    }

    pub fn target(&self) -> Option<EntityId> {
        self.target
    }

    pub fn set_target(&mut self, id: impl Into<Option<EntityId>>) {
        self.target = id.into();
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
