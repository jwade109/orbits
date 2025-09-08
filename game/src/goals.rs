use crate::game::GameState;
use starling::prelude::*;
use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Copy)]
pub struct GoalDuration {
    required: Nanotime,
    actual: Nanotime,
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
