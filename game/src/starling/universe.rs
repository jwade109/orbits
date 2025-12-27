use crate::starling::control_signals::ControlSignals;
use crate::starling::prelude::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Universe {
    stamp: Nanotime,
    ticks: u128,
    next_entity_id: EntityId,
    pub spacecraft: HashMap<EntityId, Spacecraft>,
    pub asteroids: HashMap<EntityId, Asteroid>,
    planets: PlanetarySystem,
}

impl Universe {
    pub fn empty() -> Self {
        // TODO make it so you can declare zero planets lol.
        Self::new(PlanetarySystem::Void)
    }

    pub(crate) fn new(planets: PlanetarySystem) -> Self {
        Self {
            stamp: Nanotime::zero(),
            ticks: 0,
            next_entity_id: EntityId(1002),
            spacecraft: HashMap::new(),
            asteroids: HashMap::new(),
            planets,
        }
    }

    #[deprecated]
    pub fn planets(&self) -> &PlanetarySystem {
        &self.planets
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
        self.spacecraft.remove(&id);
    }

    pub fn on_sim_ticks(
        &mut self,
        ticks: u32,
        signals: &ControlSignals,
        max_dur: Duration,
        particles: bool,
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
                self.on_sim_tick(signals, particles);
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
        self.spacecraft.iter().all(|(_, sv)| sv.can_be_on_rails())
    }

    pub fn run_batch_ticks(&mut self, ticks: u32) {
        // whatever
    }

    pub fn on_sim_tick(&mut self, signals: &ControlSignals, particles: bool) {
        // whatever
    }

    pub fn orbiter_ids(&self) -> impl Iterator<Item = EntityId> + use<'_> {
        self.spacecraft.keys().into_iter().map(|id| *id)
    }

    pub fn spawn_spacecraft(&mut self, sv: Spacecraft) -> Option<EntityId> {
        let id = self.next_entity_id();
        self.spacecraft.insert(id, sv);
        Some(id)
    }

    pub fn spawn_asteroid(&mut self, ast: Asteroid) -> Option<EntityId> {
        let id = self.next_entity_id();
        self.asteroids.insert(id, ast);
        Some(id)
    }

    pub fn add_orbital_vehicle(
        &mut self,
        vehicle: Vehicle,
        orbit: GlobalOrbit,
    ) -> Option<EntityId> {
        let id = self.next_entity_id();
        let mut body = RigidBody::random_spin();
        body.pv = orbit.1.pv(self.stamp).ok()?; // orbiter.pv(self.stamp, &self.planets)?;
        let controller = VehicleController::idle();
        let os = Spacecraft::new(orbit.0, vehicle, body, controller);
        self.spacecraft.insert(id, os);
        Some(id)
    }

    pub fn pv(&self, id: EntityId) -> Option<PV> {
        if id == EntityId(0) {
            return Some(PV::ZERO);
        }

        if let Some((_, pv, _, _)) = self.planets.lookup_planet(id, self.stamp) {
            return Some(pv);
        }

        let (local, parent) = if let Some(ov) = self.spacecraft.get(&id) {
            (ov.pv(), ov.parent())
        } else {
            return None;
        };

        let parent = self.pv(parent)?;

        Some(local + parent)
    }

    pub fn get_planet(&self, id: EntityId) -> Option<Planet> {
        let stamp = self.stamp;
        let (planet, _, _, _) = self.planets.lookup_planet(id, stamp)?;
        Some(planet)
    }

    pub fn planet_ids(&self) -> Vec<EntityId> {
        self.planets.planet_ids()
    }

    pub fn frames(&self) -> impl Iterator<Item = (PV, EntityId)> + use<'_> {
        self.spacecraft
            .iter()
            .map(|(_, ov)| (ov.pv(), ov.parent()))
            .chain(self.planets.planet_ids().into_iter().filter_map(|id| {
                let (_, _, parent, _) = self.planets.lookup_planet(id, self.stamp)?;
                let pv_child = self.pv(id)?;
                let pv_parent = self.pv(parent?)?;
                Some((pv_child - pv_parent, parent?))
            }))
    }

    pub fn get_planet_by_name(&self, name: &str) -> Option<EntityId> {
        self.planets
            .planet_ids()
            .iter()
            .filter_map(|id| {
                let planet = self.get_planet(*id)?;
                (planet.name == name).then(|| *id)
            })
            .next()
    }
}

pub fn all_orbital_ids(universe: &Universe) -> impl Iterator<Item = EntityId> + use<'_> {
    universe
        .orbiter_ids()
        .map(|id| id)
        .chain(universe.planets.planet_ids().into_iter().map(|id| id))
}

pub fn orbiters_within_bounds(
    universe: &Universe,
    bounds: AABB,
) -> impl Iterator<Item = EntityId> + use<'_> {
    universe.spacecraft.iter().filter_map(move |(id, _)| {
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
    let results = all_orbital_ids(universe)
        .filter_map(|id| {
            let pv = universe.pv(id)?;

            let planet = universe.get_planet(id);

            let size = if let Some(planet) = planet {
                planet.body.radius
            } else {
                universe
                    .spacecraft
                    .get(&id)
                    .map(|sv| sv.vehicle.bounding_radius())
                    .unwrap_or(0.0)
            };
            let p = pv.pos;
            let d = pos.distance(p);
            let passes = if let Some(m) = max_dist {
                d <= size + m
            } else {
                true
            };
            passes.then(|| (d, id))
        })
        .collect::<Vec<_>>();
    results
        .into_iter()
        .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
        .map(|(_, id)| id)
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
            let (planet, pv, _, _) = planets.lookup_planet(id, stamp)?;
            let p = pv.pos;
            let d = pos.distance(p);
            (d <= planet.body.soi).then(|| (d, id))
        })
        .collect::<Vec<_>>();
    results
        .iter()
        .min_by(|(d1, _), (d2, _)| d1.total_cmp(d2))
        .map(|(_, id)| *id)
}
