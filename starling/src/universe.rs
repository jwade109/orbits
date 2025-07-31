use crate::control_signals::ControlSignals;
use crate::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct Universe {
    stamp: Nanotime,
    ticks: u128,
    next_entity_id: EntityId,
    pub orbital_vehicles: HashMap<EntityId, OrbitalSpacecraftEntity>,
    pub surface_vehicles: HashMap<EntityId, SurfaceSpacecraftEntity>,
    pub planets: PlanetarySystem,
    pub surface: Surface,
    pub landing_sites: HashMap<EntityId, Vec<(f32, String, Surface)>>,
    pub constellations: HashMap<EntityId, EntityId>,
}

fn generate_landing_sites(pids: &[EntityId]) -> HashMap<EntityId, Vec<(f32, String, Surface)>> {
    pids.iter()
        .map(|pid| {
            let n = randint(3, 12);
            let sites: Vec<_> = (0..n)
                .map(|_| {
                    let angle = rand(0.0, 2.0 * PI);
                    let name = get_random_name();
                    (angle, name, Surface::random())
                })
                .collect();
            (*pid, sites)
        })
        .collect()
}

impl Universe {
    pub fn new(planets: PlanetarySystem) -> Self {
        Self {
            stamp: Nanotime::zero(),
            ticks: 0,
            next_entity_id: EntityId(0),
            orbital_vehicles: HashMap::new(),
            surface_vehicles: HashMap::new(),
            planets: planets.clone(),
            surface: Surface::random(),
            landing_sites: generate_landing_sites(&planets.planet_ids()),
            constellations: HashMap::new(),
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

    pub fn on_sim_ticks(&mut self, ticks: u32, signals: &ControlSignals) {
        (0..ticks).for_each(|_| self.on_sim_tick(signals));
    }

    pub fn on_sim_tick(&mut self, signals: &ControlSignals) {
        self.ticks += 1;
        self.stamp += PHYSICS_CONSTANT_DELTA_TIME;

        // for (_, orbiter) in &mut self.orbiters {
        //     orbiter.on_sim_tick();
        // }

        // for (_, vehicle) in &mut self.vehicles {
        //     vehicle.on_sim_tick();
        // }

        for (_, ov) in &mut self.orbital_vehicles {
            ov.body.on_sim_tick(
                BodyFrameAccel::default(),
                Vec2::ZERO,
                PHYSICS_CONSTANT_DELTA_TIME,
            );
        }

        let gravity = self.surface.external_acceleration();

        for (_, sv) in &mut self.surface_vehicles {
            let ext = signals.piloting.unwrap_or(VehicleControl::NULLOPT);

            let ctrl = match (sv.controller.mode(), sv.controller.get_target_pose()) {
                (VehicleControlPolicy::Idle, _) => VehicleControl::NULLOPT,
                (VehicleControlPolicy::External, _) => ext,
                (VehicleControlPolicy::PositionHold, Some(pose)) => {
                    position_hold_control_law(pose, &sv.body, &sv.vehicle, gravity)
                }
                (_, _) => VehicleControl::NULLOPT,
            };

            let elevation = self.surface.elevation(sv.body.pv.pos_f32().x);

            if !sv.controller.is_idle() {
                sv.controller
                    .check_target_achieved(&sv.body, gravity.length() > 0.0);
                sv.vehicle.set_thrust_control(ctrl);
                sv.vehicle.on_sim_tick();
            }

            if !sv.controller.is_idle() || sv.body.pv.pos_f32().y > elevation {
                let accel = sv.vehicle.body_frame_accel();
                sv.body.on_sim_tick(
                    accel,
                    self.surface.external_acceleration(),
                    PHYSICS_CONSTANT_DELTA_TIME,
                );
            } else {
                sv.vehicle.set_thrust_control(VehicleControl::NULLOPT);
            }

            sv.body.clamp_with_elevation(elevation);
        }

        self.surface.on_sim_tick();

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

    pub fn add_orbital_vehicle(&mut self, vehicle: Vehicle, orbit: GlobalOrbit) {
        let id = self.next_entity_id();
        let orbiter = Orbiter::new(orbit, self.stamp);
        let controller = OrbitalController::idle();
        let os =
            OrbitalSpacecraftEntity::new(vehicle, RigidBody::random_spin(), orbiter, controller);
        self.orbital_vehicles.insert(id, os);
    }

    pub fn add_surface_vehicle(&mut self, vehicle: Vehicle, pos: Vec2) {
        let target = pos + randvec(10.0, 20.0);
        let vel = randvec(2.0, 7.0);

        let angle = rand(0.0, PI);

        let body = RigidBody {
            pv: PV::from_f64(pos, vel),
            angle: PI / 2.0,
            angular_velocity: 0.0,
        };

        let pose: Pose = (target, angle);

        let controller = VehicleController::position_hold(pose);
        let id = self.next_entity_id();
        let sv = SurfaceSpacecraftEntity::new(vehicle, body, controller);
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
        bounds.contains(pv.pos_f32()).then(|| *id)
    })
}
