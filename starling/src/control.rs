use crate::nanotime::Nanotime;
use crate::orbiter::{ObjectId, Orbiter};
use crate::orbits::SparseOrbit;
use crate::planning::{generate_maneuver_plans, ManeuverPlan};
use crate::scenario::Scenario;
use glam::f32::Vec2;

#[derive(Debug, Clone)]
pub struct Controller {
    pub target: ObjectId,
    pub plan: Option<ManeuverPlan>,
}

impl Controller {
    pub fn new(target: ObjectId) -> Self {
        Controller { target, plan: None }
    }

    pub fn with_plan(target: ObjectId, plan: ManeuverPlan) -> Self {
        Controller {
            target,
            plan: Some(plan),
        }
    }

    pub fn target(&self) -> ObjectId {
        self.target
    }

    pub fn plan(&self) -> Option<&ManeuverPlan> {
        self.plan.as_ref()
    }
}

impl std::fmt::Display for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.target)
    }
}
