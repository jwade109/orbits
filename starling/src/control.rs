use crate::nanotime::Nanotime;
use crate::orbiter::ObjectId;
use crate::orbits::SparseOrbit;
use crate::planning::{best_maneuver_plan, ManeuverPlan};

#[derive(Debug, Clone)]
pub struct Controller {
    pub target: ObjectId,
    state: ControllerState,
}

#[derive(Debug, Clone)]
enum ControllerState {
    Idle,
    Planned {
        parent: ObjectId,
        plan: ManeuverPlan,
    },
}

impl Controller {
    pub fn idle(target: ObjectId) -> Self {
        Controller {
            target,
            state: ControllerState::Idle,
        }
    }

    pub fn clear(&mut self) {
        self.state = ControllerState::Idle;
    }

    pub fn is_idle(&self) -> bool {
        match self.state {
            ControllerState::Idle => true,
            _ => false,
        }
    }

    pub fn activate(
        &mut self,
        parent: ObjectId,
        current: &SparseOrbit,
        destination: &SparseOrbit,
        now: Nanotime,
    ) -> Option<()> {
        let plan = best_maneuver_plan(current, destination, now)?;
        self.state = ControllerState::Planned { parent, plan };
        Some(())
    }

    pub fn enqueue(&mut self, destination: &SparseOrbit) -> Result<(), &'static str> {
        let parent = self.parent().ok_or("No parent")?;
        let plan = self.plan().ok_or("No plan")?;
        let new_plan =
            best_maneuver_plan(&plan.terminal, destination, plan.end()).ok_or("No best plan")?;
        let plan = plan.then(new_plan)?;
        self.state = ControllerState::Planned { parent, plan };
        Ok(())
    }

    pub fn parent(&self) -> Option<ObjectId> {
        match &self.state {
            ControllerState::Idle => None,
            ControllerState::Planned { parent, .. } => Some(*parent),
        }
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
