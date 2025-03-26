use crate::nanotime::Nanotime;
use crate::orbiter::ObjectId;
use crate::orbits::GlobalOrbit;
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
        current: &GlobalOrbit,
        destination: &GlobalOrbit,
        now: Nanotime,
    ) -> Result<(), &'static str> {
        if current.0 != destination.0 {
            return Err("Orbits have different parents");
        }
        let plan =
            best_maneuver_plan(&current.1, &destination.1, now).ok_or("Failed to plan maneuver")?;
        self.state = ControllerState::Planned {
            parent: current.0,
            plan,
        };
        Ok(())
    }

    pub fn enqueue(&mut self, destination: &GlobalOrbit) -> Result<(), &'static str> {
        let parent = self.parent().ok_or("No parent")?;
        if parent != destination.0 {
            return Err("Different parent than existing plan");
        }
        let plan = self.plan().ok_or("No plan")?;
        let new_plan =
            best_maneuver_plan(&plan.terminal, &destination.1, plan.end()).ok_or("No best plan")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    fn simulate_navigation(init: (f32, f32, f32), dst: (f32, f32, f32)) {
        let body = Body::new(63.0, 1000.0, 10000000.0);
        let earth = PlanetarySystem::new(ObjectId(0), "Earth", body);
        let mut scenario = Scenario::new(&earth);

        let orbits = [init, dst].map(|(ra, rp, argp)| {
            SparseOrbit::new(ra, rp, argp, body, Nanotime::zero(), false).unwrap()
        });

        println!("Navigate from {}\n           to {}", orbits[0], orbits[1]);

        let orbiter_id = ObjectId(1);

        scenario.add_object(orbiter_id, earth.id, orbits[0], Nanotime::zero());

        let mut ctrl = Controller::idle(orbiter_id);

        ctrl.activate(
            &GlobalOrbit(earth.id, orbits[0]),
            &GlobalOrbit(earth.id, orbits[1]),
            Nanotime::zero(),
        )
        .unwrap();

        let mut tfinal = Nanotime::zero();

        for (t, dv) in ctrl.plan().unwrap().dvs() {
            println!("{}, {}", t, dv);
            let events = scenario.simulate(t, Nanotime::secs(10));
            assert!(events.is_empty());
            assert!(scenario.dv(orbiter_id, t, dv).is_some());
            assert!(tfinal < t);
            tfinal = t;
        }

        tfinal += Nanotime::secs(30);

        let events = scenario.simulate(tfinal, Nanotime::secs(10));
        assert!(events.is_empty());

        assert_eq!(scenario.orbiter_count(), 1);

        // this is entirely too verbose
        let lup = scenario.lup(orbiter_id, tfinal).unwrap();
        let orbiter = lup.orbiter().unwrap();
        let prop = orbiter.propagator_at(tfinal).unwrap();
        let orbit = prop.orbit;

        assert!(orbit.1.is_similar(&orbits[1]));
    }

    #[test]
    fn controller_scenarios() {
        let params = [
            (9000.0, 5000.0, -0.3),
            (10000.0, 9500.0, -0.7),
            (2000.0, 700.0, 1.2),
        ];

        for p in params {
            for q in params {
                if p == q {
                    continue;
                }
                simulate_navigation(p, q);
            }
        }
    }
}
