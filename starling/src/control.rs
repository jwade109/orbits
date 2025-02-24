use crate::scenario::Scenario;
use crate::math::{cross2d, rotate, PI};
use crate::nanotime::Nanotime;
use crate::orbiter::{ObjectId, Orbiter};
use crate::orbits::SparseOrbit;
use glam::f32::Vec2;

#[derive(Debug, Clone)]
enum ControllerMode {
    Idle,
    AvoidCollisions,
    Hohmann(SparseOrbit),
}

#[derive(Debug, Clone)]
pub struct Controller {
    target: ObjectId,
    mode: ControllerMode,
    last: Option<Vec2>,
}

impl Controller {
    pub fn idle(target: ObjectId) -> Self {
        Controller {
            target,
            mode: ControllerMode::Idle,
            last: None,
        }
    }

    pub fn hohmann(target: ObjectId, orbit: SparseOrbit) -> Self {
        Controller {
            target,
            mode: ControllerMode::Hohmann(orbit),
            last: None,
        }
    }

    pub fn avoid(target: ObjectId) -> Self {
        Controller {
            target,
            mode: ControllerMode::AvoidCollisions,
            last: None,
        }
    }

    pub fn update(&mut self, system: &Scenario, stamp: Nanotime) -> Option<Vec2> {
        self.last = match &mut self.mode {
            ControllerMode::Idle => None,
            ControllerMode::AvoidCollisions => {
                let obj = system.objects.iter().find(|o| o.id() == self.target())?;
                avoid_collisions_update_loop(obj, stamp)
            }
            ControllerMode::Hohmann(_orbit) => Some(Vec2::ZERO),
        };
        self.last
    }

    pub fn target(&self) -> ObjectId {
        self.target
    }

    pub fn last(&self) -> Option<Vec2> {
        self.last
    }
}

impl std::fmt::Display for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {:?} {:?}", self.target, self.mode, self.last)
    }
}

fn avoid_collisions_update_loop(orbiter: &Orbiter, stamp: Nanotime) -> Option<Vec2> {
    if !orbiter.will_collide() && !orbiter.has_error() {
        return None;
    }

    // don't consider future orbits yet
    if orbiter.props().len() > 1 {
        return None;
    }

    // thrust sideways!

    let p = orbiter.pvl(stamp)?;

    let strength = 0.13;
    let dir = if cross2d(p.pos, p.vel) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let prograde = rotate(p.pos.normalize_or_zero(), PI / 2.0);

    // TODO determine most effective dv to raise relevant periapsis
    // -- it could be a periapsis in a future orbit, not the current orbit

    return Some(prograde * dir * strength);
}
