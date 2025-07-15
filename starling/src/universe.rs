use crate::prelude::*;
use std::collections::HashMap;

pub struct Universe {
    stamp: Nanotime,
    ticks: u128,
    pub orbiters: HashMap<EntityId, Orbiter>,
    pub vehicles: HashMap<EntityId, Vehicle>,
    pub planets: PlanetarySystem,
}

impl Universe {
    pub fn new(planets: PlanetarySystem) -> Self {
        Self {
            stamp: Nanotime::zero(),
            ticks: 0,
            orbiters: HashMap::new(),
            vehicles: HashMap::new(),
            planets,
        }
    }

    pub fn stamp(&self) -> Nanotime {
        self.stamp
    }

    pub fn ticks(&self) -> u128 {
        self.ticks
    }

    pub fn on_sim_tick(&mut self) {
        self.ticks += 1;
        self.stamp += PHYSICS_CONSTANT_DELTA_TIME;

        for (_, orbiter) in &mut self.orbiters {
            orbiter.on_sim_tick();
        }

        for (_, vehicle) in &mut self.vehicles {
            vehicle.on_sim_tick();
        }
    }

    pub fn lup_orbiter(&self, id: EntityId, stamp: Nanotime) -> Option<ObjectLookup> {
        let orbiter = self.orbiters.get(&id)?;
        let prop = orbiter.propagator_at(stamp)?;
        let (_, frame_pv, _, _) = self.planets.lookup(prop.parent(), stamp)?;
        let local_pv = prop.pv(stamp)?;
        let pv = frame_pv + local_pv;
        Some(ObjectLookup(id, ScenarioObject::Orbiter(orbiter), pv))
    }

    pub fn lup_planet(&self, id: EntityId, stamp: Nanotime) -> Option<ObjectLookup> {
        let (body, pv, _, sys) = self.planets.lookup(id, stamp)?;
        Some(ObjectLookup(id, ScenarioObject::Body(&sys.name, body), pv))
    }
}
