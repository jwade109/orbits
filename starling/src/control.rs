use crate::nanotime::Nanotime;
use crate::orbiter::ObjectId;
use crate::orbits::SparseOrbit;
use crate::planning::{best_maneuver_plan, ManeuverPlan, ManeuverType};

#[derive(Debug, Clone)]
pub struct Controller {
    pub target: ObjectId,
    state: ControllerState,
}

#[derive(Debug, Clone)]
enum ControllerState {
    Idle,
    Planned { plan: ManeuverPlan },
}

impl Controller {
    pub fn idle(target: ObjectId) -> Self {
        Controller {
            target,
            state: ControllerState::Idle,
        }
    }

    pub fn is_idle(&self) -> bool {
        match self.state {
            ControllerState::Idle => true,
            _ => false,
        }
    }

    pub fn activate(
        &mut self,
        current: &SparseOrbit,
        destination: &SparseOrbit,
        now: Nanotime,
    ) -> Option<&mut Self> {
        let plan = best_maneuver_plan(current, destination, now)?;
        self.state = ControllerState::Planned { plan };
        Some(self)
    }

    pub fn enqueue(&mut self, destination: &SparseOrbit) -> Option<()> {
        let plan = self.plan()?;
        let (range, _, current) = plan.orbits().last()?;
        let start = range.start()?;
        let new_plan = best_maneuver_plan(current.sparse(), destination, start)?;
        let plan = self.plan()?.and_then(new_plan, ManeuverType::Compound)?;
        self.state = ControllerState::Planned { plan };
        Some(())
    }

    pub fn target(&self) -> ObjectId {
        self.target
    }

    pub fn plan(&self) -> Option<&ManeuverPlan> {
        match &self.state {
            ControllerState::Idle => None,
            ControllerState::Planned { plan, .. } => Some(plan),
        }
    }
}

impl std::fmt::Display for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.target)
    }
}
