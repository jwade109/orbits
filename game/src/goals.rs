use starling::prelude::*;

#[derive(Debug)]
pub enum GoalCondition {
    AnyLaunchToOrbit,
    AnyHoldAttitude,
    AnyManuallyControlled,
    AnyLandOnTheMoon,
    AnyLandOnEarth,
    Renzevzous(EntityId, EntityId),
}

#[derive(Debug)]
pub struct GoalDuration {
    required: Nanotime,
    actual: Nanotime,
}

#[derive(Debug)]
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
            is_permanent: true,
            cond,
            dur: None,
        }
    }

    pub fn update(&mut self, universe: &Universe) {
        if self.is_complete && self.is_permanent {
            return;
        }

        self.is_complete = is_satisfied(universe, &self.cond).unwrap_or(false);
    }

    pub fn to_string(&self) -> String {
        let s = if self.is_complete { " * " } else { " - " };
        let text = match self.cond {
            GoalCondition::AnyLaunchToOrbit => "Launch any spacecraft to orbit".to_string(),
            GoalCondition::AnyHoldAttitude => {
                "Command any spacecraft to hold current attitude".to_string()
            }
            GoalCondition::AnyManuallyControlled => {
                "Pilot any spacecraft using the arrow keys".to_string()
            }
            GoalCondition::AnyLandOnTheMoon => todo!(),
            GoalCondition::AnyLandOnEarth => todo!(),
            GoalCondition::Renzevzous(a, b) => {
                format!("Rendezvous vehicle {} with vehicle {}", a, b)
            }
        };
        format!("{}{}", s, text)
    }
}

fn is_satisfied(universe: &Universe, cond: &GoalCondition) -> Option<bool> {
    match cond {
        GoalCondition::AnyLaunchToOrbit => Some(universe.surface_vehicles.iter().any(|(_, v)| {
            if let VehicleControlPolicy::LaunchToOrbit(_) = v.controller.mode() {
                true
            } else {
                false
            }
        })),
        GoalCondition::AnyHoldAttitude => Some(universe.surface_vehicles.iter().any(|(_, v)| {
            if let VehicleControlPolicy::HoldAttitude(_) = v.controller.mode() {
                true
            } else {
                false
            }
        })),
        GoalCondition::AnyManuallyControlled => {
            Some(universe.surface_vehicles.iter().any(|(_, v)| {
                if let VehicleControlPolicy::External = v.controller.mode() {
                    true
                } else {
                    false
                }
            }))
        }
        GoalCondition::AnyLandOnTheMoon => None,
        GoalCondition::AnyLandOnEarth => None,
        GoalCondition::Renzevzous(a, b) => {
            let pva = universe.pv(*a)?;
            let pvb = universe.pv(*b)?;
            let ds = pva.pos.distance(pva.pos);
            let dv = pva.vel.distance(pvb.vel);
            Some(ds < 100.0 && dv < 10.0)
        }
    }
}

pub fn init_goals(v1: EntityId, v2: EntityId) -> impl Iterator<Item = Goal> {
    [
        Goal::new(GoalCondition::AnyHoldAttitude),
        Goal::new(GoalCondition::AnyLaunchToOrbit),
        Goal::new(GoalCondition::AnyManuallyControlled),
        Goal::new(GoalCondition::Renzevzous(v1, v2)),
    ]
    .into_iter()
}
