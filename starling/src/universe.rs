use crate::control_signals::ControlSignals;
use crate::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub struct Universe {
    stamp: Nanotime,
    ticks: u128,
    next_entity_id: EntityId,
    pub orbital_vehicles: HashMap<EntityId, OrbitalSpacecraftEntity>,
    pub surface_vehicles: HashMap<EntityId, SurfaceSpacecraftEntity>,
    pub planets: PlanetarySystem,
    pub landing_sites: HashMap<EntityId, LandingSiteEntity>,
    pub constellations: HashMap<EntityId, EntityId>,
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
        let ids = planets.planet_ids();

        let mut ret = Self {
            stamp: Nanotime::zero(),
            ticks: 0,
            next_entity_id: EntityId(1002),
            orbital_vehicles: HashMap::new(),
            surface_vehicles: HashMap::new(),
            planets: planets.clone(),
            landing_sites: HashMap::new(),
            constellations: HashMap::new(),
        };

        for ls in generate_landing_sites(&ids) {
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
        self.orbital_vehicles.remove(&id);
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
            let ls = match self.landing_sites.get_mut(&sv.surface_id) {
                Some(s) => s,
                None => continue,
            };

            let external_accel = ls.surface.body.gravity(sv.body.pv.pos);

            let ext = signals
                .piloting_commands
                .get(id)
                .map(|v| *v)
                .unwrap_or(VehicleControl::NULLOPT);

            let (ctrl, status) = match (sv.controller.mode(), sv.controller.get_target_pose()) {
                (VehicleControlPolicy::Idle, _) => {
                    (VehicleControl::NULLOPT, VehicleControlStatus::Whatever)
                }
                (VehicleControlPolicy::External, _) => (ext, VehicleControlStatus::Whatever),
                (VehicleControlPolicy::PositionHold, Some(pose)) => {
                    position_hold_control_law(pose, &sv.body, &sv.vehicle, external_accel)
                }
                (VehicleControlPolicy::LaunchToOrbit, _) => enter_orbit_control_law(
                    &ls.surface.body,
                    &sv.body,
                    &sv.vehicle,
                    sv.orbit.as_ref(),
                    300_000.0,
                ),
                (_, _) => (VehicleControl::NULLOPT, VehicleControlStatus::Whatever),
            };

            sv.controller.set_status(status);

            sv.controller
                .check_target_achieved(&sv.body, external_accel.length() > 0.0);
            sv.vehicle.set_thrust_control(ctrl);
            sv.vehicle.on_sim_tick();

            sv.orbit = SparseOrbit::from_pv(sv.body.pv, ls.surface.body, stamp);

            let accel = sv.vehicle.body_frame_accel();
            sv.body
                .on_sim_tick(accel, external_accel, PHYSICS_CONSTANT_DELTA_TIME);

            sv.body.clamp_with_elevation(ls.surface.body.radius);

            let atmo = {
                let altitude = sv.body.pv.pos.length() - ls.surface.body.radius;
                (1.0 - altitude / 200_000.0).clamp(0.0, 1.0) * ls.surface.atmo_density as f64
            };

            add_particles_from_vehicle(
                &mut ls.surface.particles,
                &sv.vehicle,
                &sv.body,
                atmo as f32,
            );

            ls.add_position_track(*id, stamp, sv.body.pv.pos);
        }
    }

    pub fn run_batch_ticks(&mut self, ticks: u32) {
        self.ticks += ticks as u128;
        let old_stamp = self.stamp;
        self.stamp = old_stamp + PHYSICS_CONSTANT_DELTA_TIME * ticks;
        let delta = self.stamp - old_stamp;

        for (_, ov) in &mut self.orbital_vehicles {
            ov.body.angle += ov.body.angular_velocity * delta.to_secs_f64();
            ov.body.angle = wrap_pi_npi_f64(ov.body.angle);
            ov.vehicle.zero_all_thrusters();
        }
    }

    pub fn on_sim_tick(&mut self, signals: &ControlSignals) {
        self.ticks += 1;
        self.stamp += PHYSICS_CONSTANT_DELTA_TIME;

        for (id, ov) in &mut self.orbital_vehicles {
            let ctrl = match signals.piloting_commands.get(id) {
                Some(ctrl) => ctrl,
                None => &VehicleControl::NULLOPT,
            };

            ov.reference_orbit_age += PHYSICS_CONSTANT_DELTA_TIME;

            if ov.reference_orbit_age > Nanotime::millis(100) {
                ov.reference_orbit_age = Nanotime::zero();
                if !ov.body.pv.is_zero() {
                    if let Some(orbit) = ov.current_orbit(self.stamp) {
                        ov.orbiter = Orbiter::new(orbit, self.stamp);
                        ov.body.pv = PV::ZERO;
                    }
                }
            }

            ov.vehicle.set_thrust_control(*ctrl);
            // ov.vehicle.on_sim_tick();

            let accel = ov.vehicle.body_frame_accel();

            ov.body
                .on_sim_tick(accel, DVec2::ZERO, PHYSICS_CONSTANT_DELTA_TIME);
        }

        self.step_surface_vehicles(signals);

        self.constellations.retain(|id, _| {
            self.orbital_vehicles.contains_key(id) || self.surface_vehicles.contains_key(id)
        });
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
        self.orbital_vehicles.keys().into_iter().map(|id| *id)
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

    pub fn add_orbital_vehicle(&mut self, vehicle: Vehicle, orbit: GlobalOrbit) {
        let id = self.next_entity_id();
        let orbiter = Orbiter::new(orbit, self.stamp);
        let controller = OrbitalController::idle();
        let os =
            OrbitalSpacecraftEntity::new(vehicle, RigidBody::random_spin(), orbiter, controller);
        self.orbital_vehicles.insert(id, os);
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

        let pos = rotate_f64(DVec2::X * (ls.surface.body.radius + altitude), angle);

        let vel = randvec(2.0, 7.0);

        let body = RigidBody {
            pv: PV::from_f64(pos, vel),
            angle: PI_64 / 2.0,
            angular_velocity: 0.0,
        };

        let controller = VehicleController::launch();
        let id = self.next_entity_id();
        let sv = SurfaceSpacecraftEntity::new(surface_id, vehicle, body, controller);
        self.surface_vehicles.insert(id, sv);
    }

    pub fn lup_orbiter(&self, id: EntityId, stamp: Nanotime) -> Option<ObjectLookup> {
        let os = self.orbital_vehicles.get(&id)?;
        let prop = os.orbiter.propagator_at(stamp)?;
        let (_, frame_pv, _, _) = self.planets.lookup(prop.parent(), stamp)?;
        let local_pv = prop.pv(stamp)?;
        let pv = frame_pv + local_pv;
        Some(ObjectLookup(id, ScenarioObject::Orbiter(&os.orbiter), pv))
    }

    pub fn lup_planet(&self, id: EntityId, stamp: Nanotime) -> Option<ObjectLookup> {
        let (body, pv, _, sys) = self.planets.lookup(id, stamp)?;
        Some(ObjectLookup(id, ScenarioObject::Body(&sys.name, body), pv))
    }

    pub fn lup_surface_vehicle(
        &self,
        id: EntityId,
        surface_id: EntityId,
    ) -> Option<&SurfaceSpacecraftEntity> {
        let sv = self.surface_vehicles.get(&id)?;
        (sv.surface_id == surface_id).then(|| sv)
    }

    pub fn surface_vehicles(
        &self,
        surface_id: EntityId,
    ) -> impl Iterator<Item = (&EntityId, &SurfaceSpacecraftEntity)> + use<'_> {
        self.surface_vehicles
            .iter()
            .filter(move |(_, sv)| sv.surface_id == surface_id)
    }

    pub fn surface_vehicles_mut(
        &mut self,
        surface_id: EntityId,
    ) -> impl Iterator<Item = (&EntityId, &mut SurfaceSpacecraftEntity)> + use<'_> {
        self.surface_vehicles
            .iter_mut()
            .filter(move |(_, sv)| sv.surface_id == surface_id)
    }

    pub fn all_surface_vehicles(
        &self,
    ) -> impl Iterator<Item = (&EntityId, &SurfaceSpacecraftEntity)> + use<'_> {
        self.surface_vehicles.iter()
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
    universe.orbital_vehicles.iter().filter_map(move |(id, _)| {
        let pv = universe.lup_orbiter(*id, universe.stamp())?.pv();
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
