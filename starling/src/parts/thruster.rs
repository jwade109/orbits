use crate::math::{rotate, Vec2};
use crate::nanotime::Nanotime;
use crate::parts::parts::ThrusterProto;

const THRUSTER_DEBOUNCE_TIME: Nanotime = Nanotime::millis(10);

#[derive(Debug, Clone)]
pub struct Thruster {
    pub proto: ThrusterProto,
    pub pos: Vec2,
    pub angle: f32,
    is_active: bool,
    last_toggled: Nanotime,
}

impl Thruster {
    pub fn new(proto: ThrusterProto, pos: Vec2, angle: f32) -> Self {
        Thruster {
            proto,
            pos,
            angle,
            is_active: false,
            last_toggled: Nanotime::zero(),
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn set_thrusting(&mut self, state: bool, stamp: Nanotime) {
        let dt = stamp - self.last_toggled;
        if dt >= THRUSTER_DEBOUNCE_TIME {
            self.is_active = state;
            self.last_toggled = stamp;
        }
    }

    pub fn toggle(&mut self, stamp: Nanotime) {
        let dt = stamp - self.last_toggled;
        if dt >= THRUSTER_DEBOUNCE_TIME {
            self.is_active = !self.is_active;
            self.last_toggled = stamp;
        }
    }

    pub fn is_thrusting(&self) -> bool {
        self.is_active
    }
}
