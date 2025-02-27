use crate::orbiter::ObjectId;
use crate::planning::ManeuverPlan;

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
