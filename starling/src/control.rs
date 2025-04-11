use crate::nanotime::Nanotime;
use crate::orbiter::ObjectId;
use crate::orbits::GlobalOrbit;
use crate::planning::{best_maneuver_plan, ManeuverPlan};

#[derive(Debug, Clone)]
pub struct Controller {
    target: ObjectId,
    last_update: Nanotime,
    current: Option<GlobalOrbit>,
    destination: Option<GlobalOrbit>,
    plan: Option<ManeuverPlan>,
}

impl Controller {
    pub fn idle(target: ObjectId) -> Self {
        Controller {
            target,
            last_update: Nanotime::zero(),
            current: None,
            destination: None,
            plan: None,
        }
    }

    pub fn clear(&mut self) {
        self.destination = None;
        self.plan = None;
    }

    pub fn is_idle(&self) -> bool {
        self.destination.is_none()
    }

    pub fn needs_update(&self, stamp: Nanotime) -> bool {
        stamp - self.last_update > Nanotime::secs(1)
    }

    pub fn update(&mut self, stamp: Nanotime, orbit: GlobalOrbit) -> Result<(), &'static str> {
        self.last_update = stamp;

        self.current = Some(orbit);

        if self.destination.is_none() {
            return Ok(());
        }

        if let Some((c, d)) = self.current.zip(self.destination) {
            if c.1.is_similar(&d.1) {
                self.destination = None;
                self.plan = None;
                return Ok(());
            }
        }

        let nominal = self
            .plan
            .as_ref()
            .map(|m| m.segment_at(stamp))
            .flatten()
            .map(|s| s.orbit);

        if let Some(nominal) = nominal {
            if orbit.1.is_similar(&nominal) {
                return Ok(());
            }
        }

        if self.current.is_some() && self.destination.is_some() {
            self.reroute(stamp)
        } else {
            Ok(())
        }
    }

    pub fn set_destination(
        &mut self,
        destination: GlobalOrbit,
        stamp: Nanotime,
    ) -> Result<(), &'static str> {
        self.destination = Some(destination);
        self.reroute(stamp)
    }

    pub fn reroute(&mut self, stamp: Nanotime) -> Result<(), &'static str> {
        let c = self.current.as_ref().ok_or("No current orbit")?;
        let d = self.destination.as_ref().ok_or("No destination orbit")?;
        if c.0 != d.0 {
            return Err("Cannot path between bodies");
        }
        let p = best_maneuver_plan(&c.1, &d.1, stamp)?;
        self.plan = Some(p);
        Ok(())
    }

    pub fn destination(&self) -> Option<&GlobalOrbit> {
        self.destination.as_ref()
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
        let c = self
            .current
            .map(|c| format!("{}", c))
            .unwrap_or("None".into());
        let d = self
            .destination
            .map(|d| format!("{}", d))
            .unwrap_or("None".into());
        write!(f, "CTRL {} {} {}", self.last_update, c, d)?;

        if let Some(p) = &self.plan {
            write!(f, "\n{}", p)?;
        }

        Ok(())
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

        assert_eq!(
            ctrl.set_destination(GlobalOrbit(earth.id, orbits[1]), Nanotime::zero()),
            Err("No current orbit")
        );

        assert_eq!(
            ctrl.update(Nanotime::zero(), GlobalOrbit(earth.id, orbits[0])),
            Ok(())
        );

        let mut tfinal = Nanotime::zero();

        for (t, dv) in ctrl.plan().unwrap().dvs() {
            println!("{}, {}", t, dv);
            let events = scenario.simulate(t, Nanotime::secs(10));
            assert!(events.is_empty());
            assert!(scenario.impulsive_burn(orbiter_id, t, dv).is_some());
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
