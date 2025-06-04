use crate::math::{rand, rotate, Vec2};
use crate::nanotime::Nanotime;
use crate::parts::parts::ThrusterProto;

const THRUSTER_DEBOUNCE_TIME: Nanotime = Nanotime::millis(10);

#[derive(Debug, Clone)]
pub struct Thruster {
    pub proto: ThrusterProto,
    pub pos: Vec2,
    pub angle: f32,
    last_toggled: Nanotime,
    throttle: f32,
}

impl Thruster {
    pub fn new(proto: ThrusterProto, pos: Vec2, angle: f32) -> Self {
        Thruster {
            proto,
            pos,
            angle,
            last_toggled: Nanotime::zero(),
            throttle: 0.0,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn set_thrusting(&mut self, throttle: f32, stamp: Nanotime) {
        self.throttle += (throttle - self.throttle) * 0.1;
        // let dt = stamp - self.last_toggled;
        // if dt >= THRUSTER_DEBOUNCE_TIME {
        //     self.is_active = state;
        //     self.last_toggled = stamp;
        // }
    }

    pub fn is_thrusting(&self) -> bool {
        self.throttle > 0.01
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }
}
