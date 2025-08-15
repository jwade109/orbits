use crate::control_signals::ControlSignals;
use crate::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub struct Universe {
    stamp: Nanotime,
    ticks: u128,
    next_entity_id: EntityId,
    pub surface_vehicles: HashMap<EntityId, SurfaceSpacecraftEntity>,
    pub planets: PlanetarySystem,
    pub landing_sites: HashMap<EntityId, LandingSiteEntity>,
    pub constellations: HashMap<EntityId, EntityId>,
    pub thrust_particles: ThrustParticleEffects,
}

fn generate_landing_sites(pids: &[EntityId]) -> Vec<LandingSiteEntity> {
    pids.iter()
        .flat_map(|pid| {
            let n = randint(3, 12);
            (0..n).map(|_| {
                let angle = rand(0.0, 2.0 * PI) as f64;
                let name = get_random_name();
                LandingSiteEntity::new(name, Surface::random(), *pid, angle)
            })
        })
        .collect()
}

impl Universe {
    pub fn new(planets: PlanetarySystem) -> Self {
        let mut ret = Self {
            stamp: Nanotime::zero(),
            ticks: 0,
            next_entity_id: EntityId(1002),
            surface_vehicles: HashMap::new(),
            planets: planets.clone(),
            landing_sites: HashMap::new(),
            constellations: HashMap::new(),
            thrust_particles: ThrustParticleEffects::new(),
        };

        for ls in generate_landing_sites(&[EntityId(1)]) {
            ret.add_landing_site(ls.name, ls.surface, ls.planet, ls.angle);
        }

        ret
    }

    pub fn stamp(&self) -> Nanotime {
        self.stamp
    }

    pub fn ticks(&self) -> u128 {
        self.ticks
    }

    fn next_entity_id(&mut self) -> EntityId {
        let ret = self.next_entity_id;
        self.next_entity_id.0 += 1;
        ret
    }

    pub fn remove(&mut self, id: EntityId) {
        self.surface_vehicles.remove(&id);
        self.landing_sites.iter_mut().for_each(|(_, ls)| {
            ls.tracks.remove(&id);
        });
    }

    pub fn on_sim_ticks(
        &mut self,
        ticks: u32,
        signals: &ControlSignals,
        max_dur: Duration,
        batch_mode: bool,
    ) -> (u32, Duration, bool) {
        let start = Instant::now();
        let mut actual_ticks = 0;
        let mut exec_time = Duration::ZERO;

        let batch_mode = if batch_mode {
            self.run_batch_ticks(ticks);
            exec_time = std::time::Instant::now() - start;
            actual_ticks = ticks;
            true
        } else {
            for _ in 0..ticks {
                actual_ticks += 1;
                self.on_sim_tick(signals);
                exec_time = std::time::Instant::now() - start;
                if exec_time > max_dur {
                    break;
                }
            }
            false
        };

        (actual_ticks, exec_time, batch_mode)
    }

    fn step_surface_vehicles(&mut self, signals: &ControlSignals) {
        for (_, ls) in &mut self.landing_sites {
            ls.surface.particles.step();
        }

        let stamp = self.stamp();

        for (id, sv) in &mut self.surface_vehicles {
            let parent_body = match self.planets.lookup(sv.planet_id, self.stamp) {
                Some((body, _, _, _)) => body,
                None => continue,
            };

            let gravity = parent_body.gravity(sv.body.pv.pos);

            let ext = signals
                .piloting_commands
                .get(id)
                .map(|v| *v)
                .unwrap_or(VehicleControl::NULLOPT);

            let (ctrl, status) = match (sv.controller.mode(), sv.controller.get_target_pose()) {
                (VehicleControlPolicy::Idle, _) => {
                    (VehicleControl::NULLOPT, VehicleControlStatus::Idling)
                }
                (VehicleControlPolicy::External, _) => (
                    ext,
                    if ext == VehicleControl::NULLOPT {
                        VehicleControlStatus::WaitingForInput
                    } else {
                        VehicleControlStatus::UnderExternalControl
                    },
                ),
                (VehicleControlPolicy::PositionHold(_), Some(pose)) => {
                    position_hold_control_law(pose, &sv.body, &sv.vehicle, gravity)
                }
                (VehicleControlPolicy::LaunchToOrbit(altitude), _) => enter_orbit_control_law(
                    &parent_body,
                    &sv.body,
                    &sv.vehicle,
                    sv.orbit.as_ref(),
                    *altitude,
                ),
                (_, _) => (VehicleControl::NULLOPT, VehicleControlStatus::Whatever),
            };

            sv.controller.set_status(status);

            sv.controller
                .check_target_achieved(&sv.body, gravity.length() > 0.0);
            sv.vehicle.set_thrust_control(&ctrl);
            sv.vehicle.on_sim_tick();

            let altitude = sv.body.pv.pos.length() - parent_body.radius;

            sv.orbit = if altitude > 2_000.0 {
                SparseOrbit::from_pv(sv.body.pv, parent_body, stamp)
            } else {
                None
            };

            let accel = sv.vehicle.body_frame_accel();
            sv.body
                .on_sim_tick(accel, gravity, PHYSICS_CONSTANT_DELTA_TIME);

            sv.body.clamp_with_elevation(parent_body.radius);

            let atmo = {
                let altitude = sv.body.pv.pos.length() - parent_body.radius;
                (1.0 - altitude / 200_000.0).clamp(0.0, 1.0)
            };

            add_particles_from_vehicle(
                &mut self.thrust_particles,
                &sv.vehicle,
                &sv.body,
                atmo as f32,
            );

            // ls.add_position_track(*id, stamp, sv.body.pv.pos);
        }
    }

    pub fn run_batch_ticks(&mut self, ticks: u32) {
        self.ticks += ticks as u128;
        let old_stamp = self.stamp;
        self.stamp = old_stamp + PHYSICS_CONSTANT_DELTA_TIME * ticks;
        let delta = self.stamp - old_stamp;
    }

    pub fn on_sim_tick(&mut self, signals: &ControlSignals) {
        self.ticks += 1;
        self.stamp += PHYSICS_CONSTANT_DELTA_TIME;

        self.thrust_particles.step();

        self.step_surface_vehicles(signals);

        self.constellations
            .retain(|id, _| self.surface_vehicles.contains_key(id));
    }

    pub fn get_group_members(&mut self, gid: EntityId) -> Vec<EntityId> {
        self.constellations
            .iter()
            .filter_map(|(id, g)| (*g == gid).then(|| *id))
            .collect()
    }

    pub fn group_membership(&self, id: &EntityId) -> Option<EntityId> {
        self.constellations.get(id).cloned()
    }

    pub fn unique_groups(&self) -> Vec<EntityId> {
        let mut s: Vec<EntityId> = self
            .constellations
            .iter()
            .map(|(_, gid)| *gid)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        s.sort();
        s
    }

    pub fn orbiter_ids(&self) -> impl Iterator<Item = EntityId> + use<'_> {
        self.surface_vehicles.keys().into_iter().map(|id| *id)
    }

    pub fn add_landing_site(
        &mut self,
        name: String,
        surface: Surface,
        planet: EntityId,
        angle: f64,
    ) {
        let id = self.next_entity_id();
        let entity = LandingSiteEntity::new(name, surface, planet, angle);
        self.landing_sites.insert(id, entity);
    }

    pub fn add_orbital_vehicle(&mut self, vehicle: Vehicle, orbit: GlobalOrbit) -> Option<()> {
        let id = self.next_entity_id();
        let mut body = RigidBody::random_spin();
        body.pv = orbit.1.pv(self.stamp).ok()?; // orbiter.pv(self.stamp, &self.planets)?;
        let controller = VehicleController::idle();
        let os = SurfaceSpacecraftEntity::new(orbit.0, vehicle, body, controller);
        self.surface_vehicles.insert(id, os);
        Some(())
    }

    pub fn add_surface_vehicle(
        &mut self,
        surface_id: EntityId,
        vehicle: Vehicle,
        angle: f64,
        altitude: f64,
    ) {
        let ls = match self.landing_sites.get(&surface_id) {
            Some(ls) => ls,
            None => return,
        };

        let planet_id = ls.planet;

        let pos = rotate_f64(DVec2::X * (ls.surface.body.radius + altitude), angle);

        let vel = randvec(2.0, 7.0);

        let body = RigidBody {
            pv: PV::from_f64(pos, vel),
            angle: PI_64 / 2.0,
            angular_velocity: 0.0,
        };

        let controller = VehicleController::launch();
        let id = self.next_entity_id();
        let sv = SurfaceSpacecraftEntity::new(planet_id, vehicle, body, controller);
        self.surface_vehicles.insert(id, sv);
    }

    #[deprecated]
    pub fn lup_orbiter(&self, id: EntityId, stamp: Nanotime) -> Option<ObjectLookup> {
        let os = self.surface_vehicles.get(&id)?;
        let pv = os.pv();
        let (_, frame_pv, _, _) = self.planets.lookup(os.parent(), stamp)?;
        let pv = frame_pv + pv;
        Some(ObjectLookup(id, ScenarioObject::Orbiter(os), pv))
    }

    pub fn pv(&self, id: EntityId) -> Option<PV> {
        if let Some((_, pv, _, _)) = self.planets.lookup(id, self.stamp) {
            return Some(pv);
        }

        let (local, parent) = if let Some(ov) = self.surface_vehicles.get(&id) {
            (ov.pv(), ov.parent())
        } else {
            return None;
        };

        let (_, parent, _, _) = self.planets.lookup(parent, self.stamp)?;

        Some(local + parent)
    }

    pub fn lup_planet(&self, id: EntityId, stamp: Nanotime) -> Option<ObjectLookup> {
        let (body, pv, _, sys) = self.planets.lookup(id, stamp)?;
        Some(ObjectLookup(id, ScenarioObject::Body(&sys.name, body), pv))
    }

    pub fn frames(&self) -> impl Iterator<Item = (PV, EntityId)> + use<'_> {
        self.surface_vehicles
            .iter()
            .map(|(_, ov)| (ov.pv(), ov.parent()))
            .chain(self.planets.planet_ids().into_iter().filter_map(|id| {
                let (_, pv, parent, _) = self.planets.lookup(id, self.stamp)?;
                Some((pv, parent?))
            }))
    }
}

pub fn all_orbital_ids(universe: &Universe) -> impl Iterator<Item = ObjectId> + use<'_> {
    universe
        .orbiter_ids()
        .map(|id| ObjectId::Orbiter(id))
        .chain(
            universe
                .planets
                .planet_ids()
                .into_iter()
                .map(|id| ObjectId::Planet(id)),
        )
}

pub fn orbiters_within_bounds(
    universe: &Universe,
    bounds: AABB,
) -> impl Iterator<Item = EntityId> + use<'_> {
    universe.surface_vehicles.iter().filter_map(move |(id, _)| {
        let pv = universe.pv(*id)?;
        bounds.contains(aabb_stopgap_cast(pv.pos)).then(|| *id)
    })
}

pub fn nearest(universe: &Universe, pos: DVec2) -> Option<ObjectId> {
    let stamp = universe.stamp();
    let results = all_orbital_ids(universe)
        .filter_map(|id| {
            let lup = match id {
                ObjectId::Orbiter(id) => universe.lup_orbiter(id, stamp),
                ObjectId::Planet(id) => universe.lup_planet(id, stamp),
            }?;
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

pub fn landing_site_position(
    universe: &Universe,
    planet_id: EntityId,
    angle: f64,
) -> Option<DVec2> {
    let lup = universe.lup_planet(planet_id, universe.stamp())?;
    let body = lup.body()?;
    let center = lup.pv().pos;
    Some(center + rotate_f64(DVec2::X * body.radius, angle))
}
