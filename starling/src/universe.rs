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
    pub constellations: HashMap<EntityId, EntityId>,
    pub thrust_particles: ThrustParticleEffects,
}

impl Universe {
    pub fn empty() -> Self {
        // TODO make it so you can declare zero planets lol.
        Self::new(PlanetarySystem::new(EntityId(0), "null", Body::LUNA))
    }

    pub fn new(planets: PlanetarySystem) -> Self {
        Self {
            stamp: Nanotime::zero(),
            ticks: 0,
            next_entity_id: EntityId(1002),
            surface_vehicles: HashMap::new(),
            planets,
            constellations: HashMap::new(),
            thrust_particles: ThrustParticleEffects::new(),
        }
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

        let batch_mode = if self.can_run_batch_mode() && signals.is_empty() {
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

    fn can_run_batch_mode(&self) -> bool {
        self.surface_vehicles
            .iter()
            .all(|(_, sv)| sv.can_be_on_rails())
    }

    fn step_surface_vehicles(&mut self, signals: &ControlSignals) {
        let stamp = self.stamp();

        for (id, sv) in &mut self.surface_vehicles {
            let ext = *signals
                .piloting_commands
                .get(&id)
                .unwrap_or(&VehicleControl::NULLOPT);

            sv.step(&self.planets, stamp, ext);

            let atmo = match self.planets.lookup(sv.parent(), stamp) {
                Some((body, _, _, _)) => {
                    let altitude = sv.body.pv.pos.length() - body.radius;
                    (1.0 - altitude / 200_000.0).clamp(0.0, 1.0)
                }
                _ => 0.0,
            };

            add_particles_from_vehicle(
                &mut self.thrust_particles,
                sv.planet_id,
                &sv.vehicle,
                &sv.body,
                atmo as f32,
            );
        }
    }

    pub fn run_batch_ticks(&mut self, ticks: u32) {
        self.ticks += ticks as u128;
        let old_stamp = self.stamp;
        let delta_time = PHYSICS_CONSTANT_DELTA_TIME * ticks;
        self.stamp = old_stamp + delta_time;

        for (_, sv) in &mut self.surface_vehicles {
            sv.step_on_rails(delta_time, self.stamp, &self.planets);
        }
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
        planet_id: EntityId,
        vehicle: Vehicle,
        angle: f64,
        altitude: f64,
    ) -> Option<EntityId> {
        let lup = self.lup_planet(planet_id, self.stamp)?;
        let body = lup.body()?;

        let pos = rotate_f64(DVec2::X * (body.radius + altitude), angle);

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

        Some(id)
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

    pub fn lup_planet_by_name(&self, name: &str) -> Option<EntityId> {
        self.planets
            .planet_ids()
            .iter()
            .filter_map(|id| {
                let lup = self.lup_planet(*id, self.stamp)?;
                Some((*id, lup.named_body()?.0))
            })
            .find(|s| s.1 == name)
            .map(|s| s.0)
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

pub fn nearest_orbiter_or_planet(
    universe: &Universe,
    pos: DVec2,
    max_dist: impl Into<Option<f64>>,
) -> Option<EntityId> {
    let max_dist = max_dist.into();
    let stamp = universe.stamp();
    let results = all_orbital_ids(universe)
        .filter_map(|id| {
            let lup = match id {
                ObjectId::Orbiter(id) => universe.lup_orbiter(id, stamp),
                ObjectId::Planet(id) => universe.lup_planet(id, stamp),
            }?;
            let size = if let Some(body) = lup.body() {
                body.radius
            } else {
                0.0
            };
            let p = lup.pv().pos;
            let d = pos.distance(p);
            let passes = if let Some(m) = max_dist {
                d <= size + m
            } else {
                true
            };
            passes.then(|| (d, id.as_eid()))
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

pub fn nearest_relevant_body(
    planets: &PlanetarySystem,
    pos: DVec2,
    stamp: Nanotime,
) -> Option<EntityId> {
    let results = planets
        .planet_ids()
        .into_iter()
        .filter_map(|id| {
            let (body, pv, _, _) = planets.lookup(id, stamp)?;
            let p = pv.pos;
            let d = pos.distance(p);
            (d <= body.soi).then(|| (d, id))
        })
        .collect::<Vec<_>>();
    results
        .iter()
        .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
        .map(|(_, id)| *id)
}
