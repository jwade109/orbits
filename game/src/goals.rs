use crate::camera_controller::camera_span_meters;
use crate::game::GameState;
use serde::{Deserialize, Serialize};
use starling::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum GoalCondition {
    AnyLaunchToOrbit,
    AnyHoldAttitude,
    AnyManuallyControlled,
    AnyLandOnTheMoon,
    AnyLandOnEarth,
    Rendezvous { ownship: EntityId, target: EntityId },
    SetTarget { ownship: EntityId, target: EntityId },
    SelectVehicle(EntityId),
    ThrustForward(EntityId),
    ThrustBackward(EntityId),
    HoldAttitude(EntityId, u16),
    FocusCamera(EntityId),
    ZoomCameraTo(f64, f64),
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
                gd.actual += PHYSICS_CONSTANT_DELTA_TIME;
                self.is_complete = gd.actual >= gd.required;
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
        GoalCondition::AnyLaunchToOrbit => {
            Some(state.universe.surface_vehicles.iter().any(|(_, v)| {
                if let VehicleControlPolicy::LaunchToOrbit(_) = v.controller.mode() {
                    true
                } else {
                    false
                }
            }))
        }
        GoalCondition::AnyHoldAttitude => {
            Some(state.universe.surface_vehicles.iter().any(|(_, v)| {
                if let VehicleControlPolicy::HoldAttitude(_) = v.controller.mode() {
                    true
                } else {
                    false
                }
            }))
        }
        GoalCondition::AnyManuallyControlled => {
            Some(state.universe.surface_vehicles.iter().any(|(_, v)| {
                if let VehicleControlPolicy::External = v.controller.mode() {
                    true
                } else {
                    false
                }
            }))
        }
        GoalCondition::AnyLandOnTheMoon => None,
        GoalCondition::AnyLandOnEarth => None,
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
            let sv = state.universe.surface_vehicles.get(id)?;
            let heading = *heading as f64 / 180.0 * PI_64;
            let error_deg = wrap_pi_npi_f64(sv.body.angle - heading).abs().to_degrees();
            let rate = sv.body.angular_velocity.to_degrees();
            Some(error_deg.abs() < 5.0 && rate.abs() < 3.0)
        }
        GoalCondition::FocusCamera(id) => Some(state.orbital_context.camera.is_following(*id)),
        GoalCondition::ZoomCameraTo(min, max) => {
            let meters = camera_span_meters(
                state.input.screen_bounds.span,
                &state.orbital_context.camera,
            );

            Some(*min <= meters.length() && meters.length() <= *max)
        }
    }
}

pub fn init_goals(v1: EntityId, v2: EntityId) -> impl Iterator<Item = Goal> {
    [
        Goal::new(GoalCondition::AnyHoldAttitude),
        Goal::new(GoalCondition::AnyLaunchToOrbit),
        Goal::new(GoalCondition::AnyManuallyControlled),
        Goal::new(GoalCondition::SelectVehicle(v1)).as_impermanent(),
        Goal::new(GoalCondition::SetTarget {
            ownship: v1,
            target: v2,
        }),
        Goal::new(GoalCondition::Rendezvous {
            ownship: v1,
            target: v2,
        })
        .with_duration(Nanotime::secs(10)),
    ]
    .into_iter()
}
