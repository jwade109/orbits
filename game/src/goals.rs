use crate::camera_controller::camera_span_meters;
use crate::game::GameState;
use crate::sim_rate::SimRate;
use serde::{Deserialize, Serialize};
use starling::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum GoalCondition {
    Rendezvous {
        ownship: EntityId,
        target: EntityId,
    },
    SetTarget {
        ownship: EntityId,
        target: EntityId,
    },
    SelectVehicle(EntityId),
    ThrustForward(EntityId),
    ThrustBackward(EntityId),
    HoldAttitude(EntityId, u16),
    FocusCamera(EntityId),
    ZoomCamera,
    Apoapsis(EntityId, f64, f64),
    Periapsis(EntityId, f64, f64),
    Orbit {
        vehicle_id: EntityId,
        planet_id: EntityId,
        rp: f64,
        ra: f64,
        argp: f64,
        tol: f64,
    },
    Prograde(EntityId),
    Retrograde(EntityId),
    Idle(EntityId),
    TimeWarp,
    CancelTimeWarp,
    Pause,
}

#[derive(Debug, Clone, Copy)]
pub struct GoalDuration {
    pub required: Nanotime,
    pub actual: Nanotime,
}

#[derive(Debug, Clone, Copy)]
pub struct Goal {
    pub is_complete: bool,
    pub is_permanent: bool,
    pub cond: GoalCondition,
    pub dur: Option<GoalDuration>,
}

impl Goal {
    pub fn new(cond: GoalCondition) -> Self {
        Self {
            is_complete: false,
            is_permanent: false,
            cond,
            dur: None,
        }
    }

    pub fn with_duration(mut self, duration: Nanotime) -> Self {
        self.dur = Some(GoalDuration {
            required: duration,
            actual: Nanotime::ZERO,
        });
        self
    }

    pub fn as_impermanent(mut self) -> Self {
        self.is_permanent = false;
        self
    }

    pub fn update(&mut self, state: &GameState) {
        if self.is_complete && self.is_permanent {
            return;
        }

        let satisfied = is_satisfied(state, &self.cond).unwrap_or(false);

        if let Some(gd) = &mut self.dur {
            if satisfied {
                if !state.paused {
                    gd.actual += PHYSICS_CONSTANT_DELTA_TIME;
                    self.is_complete = gd.actual >= gd.required;
                }
            } else {
                gd.actual = Nanotime::ZERO;
                self.is_complete = false;
            }
        } else {
            self.is_complete = satisfied;
        }
    }

    pub fn progress(&self) -> f32 {
        if let Some(gd) = &self.dur {
            gd.actual.to_secs() / gd.required.to_secs()
        } else {
            self.is_complete as u8 as f32
        }
    }
}

fn is_satisfied(state: &GameState, cond: &GoalCondition) -> Option<bool> {
    match cond {
        GoalCondition::Rendezvous { ownship, target } => {
            let pva = state.universe.pv(*ownship)?;
            let pvb = state.universe.pv(*target)?;
            let ds = pva.pos.distance(pva.pos);
            let dv = pva.vel.distance(pvb.vel);
            Some(ds < 100.0 && dv < 3.0)
        }
        GoalCondition::SetTarget { ownship, target } => {
            let sv = state.universe.surface_vehicles.get(ownship)?;
            Some(sv.target() == Some(*target))
        }
        GoalCondition::SelectVehicle(v) => Some(state.orbital_context.piloting == Some(*v)),
        GoalCondition::ThrustForward(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            Some(sv.vehicle.is_thrusting() && sv.vehicle.body_frame_accel().linear.x > 2.0)
        }
        GoalCondition::ThrustBackward(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            Some(sv.vehicle.is_thrusting() && sv.vehicle.body_frame_accel().linear.x < -2.0)
        }
        GoalCondition::HoldAttitude(id, heading) => {
            if Some(*id) != state.piloting() {
                return Some(false);
            }
            let sv = state.universe.surface_vehicles.get(id)?;
            let heading = *heading as f64 / 180.0 * PI_64;
            let error_deg = wrap_pi_npi_f64(sv.body.angle - heading).abs().to_degrees();
            let rate = sv.body.angular_velocity.to_degrees();
            Some(error_deg.abs() < 5.0 && rate.abs() < 3.0)
        }
        GoalCondition::FocusCamera(id) => Some(state.orbital_context.camera.is_following(*id)),
        GoalCondition::ZoomCamera => {
            let min = 5.0;
            let max = 250.0;
            let meters = camera_span_meters(
                state.input.screen_bounds.span,
                &state.orbital_context.camera,
            );

            Some(min <= meters.length() && meters.length() <= max)
        }
        GoalCondition::Periapsis(id, min, max) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let orbit = sv.current_orbit()?;
            let r = orbit.1.periapsis_r();
            Some(*min <= r && r <= *max)
        }
        GoalCondition::Apoapsis(id, min, max) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let orbit = sv.current_orbit()?;
            let r = orbit.1.apoapsis_r();
            Some(*min <= r && r <= *max)
        }
        GoalCondition::Orbit {
            vehicle_id,
            planet_id,
            rp,
            ra,
            argp,
            tol,
        } => {
            let sv = state.universe.surface_vehicles.get(vehicle_id)?;
            if sv.parent() != *planet_id {
                return Some(false);
            }
            let orbit = sv.current_orbit()?;
            let target_orbit =
                SparseOrbit::new(*ra, *rp, *argp, orbit.1.body, Nanotime::ZERO, false)?;
            let ta = target_orbit.apoapsis();
            let tp = target_orbit.periapsis();
            let a = orbit.1.apoapsis();
            let p = orbit.1.periapsis();
            Some(a.distance(ta) < *tol && p.distance(tp) < *tol)
        }
        GoalCondition::Prograde(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            Some(
                sv.controller.mode().is_prograde()
                    && sv.controller.status() != VehicleControlStatus::ComingAbout,
            )
        }
        GoalCondition::Retrograde(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            Some(
                sv.controller.mode().is_retrograde()
                    && sv.controller.status() != VehicleControlStatus::ComingAbout,
            )
        }
        GoalCondition::Idle(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            Some(sv.controller.mode().is_idle())
        }
        GoalCondition::TimeWarp => Some(
            state.universe_ticks_per_game_tick.as_ticks() >= SimRate::FiveMinsPerSecond.as_ticks(),
        ),
        GoalCondition::CancelTimeWarp => {
            Some(state.universe_ticks_per_game_tick == SimRate::RealTime)
        }
        GoalCondition::Pause => Some(state.paused),
    }
}

pub fn get_goal_text(cond: &GoalCondition, state: &GameState) -> Option<String> {
    match cond {
        GoalCondition::SelectVehicle(id) => {
            let sv = state.universe.surface_vehicles.get(&id)?;
            let title = sv.vehicle.title_with_id(*id);
            Some(format!(
                "Select vessel \"{}\" by left-clicking on it.",
                title
            ))
        }
        GoalCondition::SetTarget { ownship, target } => {
            let ov = state.universe.surface_vehicles.get(ownship)?;
            let tv = state.universe.surface_vehicles.get(target)?;
            let ot = ov.vehicle.name();
            let tt = tv.vehicle.title_with_id(*target);
            Some(format!(
                "Set the target of \"{}\" to \"{}\" by right-clicking on it.",
                ot, tt,
            ))
        }
        GoalCondition::Rendezvous { ownship, target } => {
            let ov = state.universe.surface_vehicles.get(ownship)?;
            let tv = state.universe.surface_vehicles.get(target)?;
            let ot = ov.vehicle.name();
            let tt = tv.vehicle.name();
            Some(format!("Rendezvous \"{}\" with \"{}\".", ot, tt,))
        }
        GoalCondition::ThrustForward(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Apply forwards thrust to \"{}\".", n))
        }
        GoalCondition::ThrustBackward(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Apply backwards thrust to \"{}\".", n))
        }
        GoalCondition::HoldAttitude(id, deg) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Hold heading of \"{}\" at {} degrees.", n, deg))
        }
        GoalCondition::FocusCamera(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!(
                "Set the camera to follow \"{}\" with ctrl+left-click.",
                n
            ))
        }
        GoalCondition::ZoomCamera => Some(format!("Press V to zoom in on the selected vehicle.",)),
        GoalCondition::Periapsis(_, min, max) => Some(format!(
            "Set periapsis to between {} and {}.",
            distance_str(*min),
            distance_str(*max)
        )),
        GoalCondition::Apoapsis(_, min, max) => Some(format!(
            "Set apoapsis to between {} and {}.",
            distance_str(*min),
            distance_str(*max)
        )),
        GoalCondition::Orbit {
            vehicle_id,
            planet_id,
            ..
        } => {
            let sv = state.universe.surface_vehicles.get(vehicle_id)?;
            let n = sv.vehicle.name_with_id(*vehicle_id);
            let lup = state.universe.lup_planet(*planet_id)?;
            let b = lup.named_body()?.0;
            Some(format!(
                "Place \"{}\" into the specified orbit around {}.",
                n, b
            ))
        }
        GoalCondition::Prograde(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Switch mode for \"{}\" to PROGRADE.", n))
        }
        GoalCondition::Retrograde(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Switch mode for \"{}\" to RETROGRADE.", n))
        }
        GoalCondition::Idle(id) => {
            let sv = state.universe.surface_vehicles.get(id)?;
            let n = sv.vehicle.name_with_id(*id);
            Some(format!("Switch mode for \"{}\" to IDLE.", n))
        }
        GoalCondition::TimeWarp => Some(format!("Accelerate time using time warp by pressing [.]")),
        GoalCondition::CancelTimeWarp => {
            Some(format!("Cancel time warp with [/] to return to real time."))
        }
        GoalCondition::Pause => Some(format!("Pause the game with [SPACE].")),
    }
}
