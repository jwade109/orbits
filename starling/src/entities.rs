use crate::prelude::*;

#[derive(Debug)]
pub struct SurfaceSpacecraftEntity {
    pub planet_id: EntityId,
    pub vehicle: Vehicle,
    pub body: RigidBody,
    pub controller: VehicleController,
    pub orbit: Option<SparseOrbit>,
    pub reference_orbit_age: Nanotime,
    target: Option<EntityId>,
    orbiter: Option<Orbiter>,
}

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
            orbiter: None,
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

    pub fn props(&self) -> impl Iterator<Item = &Propagator> + use<'_> {
        self.orbiter.iter().flat_map(|o| o.props())
    }

    pub fn step_on_rails(
        &mut self,
        delta_time: Nanotime,
        stamp: Nanotime,
        planets: &PlanetarySystem,
    ) {
        if let Some(pv) = &self.orbit.map(|o| o.pv(stamp).ok()).flatten() {
            self.body.pv = *pv;
        } else {
            let accel = BodyFrameAccel {
                linear: DVec2::ZERO,
                angular: 0.0,
            };
            self.body.on_sim_tick(accel, DVec2::ZERO, delta_time);
        }

        self.body.angle += self.body.angular_velocity * delta_time.to_secs_f64();
        self.body.angle = wrap_0_2pi_f64(self.body.angle);

        let (_, parent_pv) = match planets.lookup(self.planet_id, stamp) {
            Some((body, pv, _, _)) => (body, pv),
            None => todo!(),
        };

        self.reparent_if_necessary(parent_pv, planets, stamp);
    }

    fn reparent_to(
        &mut self,
        new_parent: EntityId,
        planets: &PlanetarySystem,
        stamp: Nanotime,
    ) -> Option<()> {
        println!("Reparent from {} to {}", self.planet_id, new_parent);
        let (_, old_parent_pv, _, _) = planets.lookup(self.planet_id, stamp)?;
        let (_, new_parent_pv, _, _) = planets.lookup(new_parent, stamp)?;
        self.body.pv += old_parent_pv - new_parent_pv;
        self.planet_id = new_parent;
        Some(())
    }

    fn reparent_if_necessary(
        &mut self,
        parent_pv: PV,
        planets: &PlanetarySystem,
        stamp: Nanotime,
    ) -> Option<()> {
        let pv = self.body.pv + parent_pv;
        let new_parent_id = nearest_relevant_body(planets, pv.pos, stamp)?;
        if new_parent_id == self.planet_id {
            return None;
        }

        let (new_parent_body, _, _, _) = planets.lookup(new_parent_id, stamp)?;
        self.reparent_to(new_parent_id, planets, stamp)?;
        let altitude = self.body.pv.pos.length() - new_parent_body.radius;
        self.update_orbit(planets, altitude, new_parent_body, stamp);
        Some(())
    }

    pub fn step(&mut self, planets: &PlanetarySystem, stamp: Nanotime, ext: VehicleControl) {
        let (parent_body, parent_pv) = match planets.lookup(self.planet_id, stamp) {
            Some((body, pv, _, _)) => (body, pv),
            None => todo!(),
        };

        let gravity = parent_body.gravity(self.body.pv.pos);

        let (ctrl, status) = match (self.controller.mode(), self.controller.get_target_pose()) {
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
                position_hold_control_law(pose, &self.body, &self.vehicle, gravity)
            }
            (VehicleControlPolicy::LaunchToOrbit(altitude), _) => enter_orbit_control_law(
                &parent_body,
                &self.body,
                &self.vehicle,
                self.orbit.as_ref(),
                *altitude,
            ),
            (VehicleControlPolicy::BurnPrograde, _) => {
                burn_along_velocity_vector_control_law(&self.body, &self.vehicle, true)
            }
            (VehicleControlPolicy::BurnRetrograde, _) => {
                burn_along_velocity_vector_control_law(&self.body, &self.vehicle, false)
            }
            (_, _) => (VehicleControl::NULLOPT, VehicleControlStatus::Idling),
        };

        self.controller.set_status(status);

        if status.is_done() {
            self.controller.set_idle();
        }

        if status.is_awaiting_user_input() && ext == VehicleControl::NULLOPT {
            self.controller.set_idle();
        }

        if ext != VehicleControl::NULLOPT {
            self.controller = VehicleController::external();
        }

        self.controller
            .check_target_achieved(&self.body, gravity.length() > 0.0);
        self.vehicle.set_thrust_control(&ctrl);
        self.vehicle.on_sim_tick();

        let altitude = self.body.pv.pos.length() - parent_body.radius;

        let accel = self.vehicle.body_frame_accel();
        self.body
            .on_sim_tick(accel, gravity, PHYSICS_CONSTANT_DELTA_TIME);

        self.body.clamp_with_elevation(parent_body.radius);

        self.reparent_if_necessary(parent_pv, planets, stamp);

        self.update_orbit(planets, altitude, parent_body, stamp);
    }

    fn update_orbit(
        &mut self,
        _planets: &PlanetarySystem,
        altitude: f64,
        parent_body: Body,
        stamp: Nanotime,
    ) {
        self.orbit = if altitude > 2_000.0 {
            SparseOrbit::from_pv(self.body.pv, parent_body, stamp)
        } else {
            None
        };

        // if let Some(orbit) = self.current_orbit() {
        //     let mut orbiter = Orbiter::new(orbit, stamp);
        //     if let Err(e) = orbiter.propagate_to(stamp, Nanotime::days(3), planets) {
        //         dbg!(e);
        //     }
        //     self.orbiter = Some(orbiter);
        // }
    }

    pub fn can_be_on_rails(&self) -> bool {
        match (self.controller.mode(), self.controller.status()) {
            (VehicleControlPolicy::Idle, VehicleControlStatus::Idling) => true,
            _ => false,
        }
    }
}
