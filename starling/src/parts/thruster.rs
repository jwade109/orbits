use crate::math::{rotate, Vec2};
use crate::nanotime::Nanotime;
use crate::parts::parts::ThrusterProto;

#[derive(Debug, Clone)]
pub struct Thruster {
    pub proto: ThrusterProto,
    pub pos: Vec2,
    pub angle: f32,
    stamp: Nanotime,
    throttle_rate: f32,
    throttle: f32,
}

pub const THRUSTER_DEAD_BAND: f32 = 0.10; // minimum 10 percent throttle

impl Thruster {
    pub fn new(proto: ThrusterProto, pos: Vec2, angle: f32) -> Self {
        Thruster {
            proto,
            pos,
            angle,
            stamp: Nanotime::zero(),
            throttle_rate: 12.0,
            throttle: 0.0,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn set_thrusting(&mut self, throttle: f32, stamp: Nanotime) {
        let dt = stamp - self.stamp;
        self.stamp = stamp;
        let dthrottle = (self.throttle_rate * dt.to_secs()).abs();
        let diff = (throttle - self.throttle).abs();
        if self.throttle < throttle {
            self.throttle += dthrottle.min(diff);
        } else if self.throttle > throttle {
            self.throttle -= dthrottle.min(diff);
        }
        self.throttle = self.throttle.clamp(0.0, 1.0);
    }

    pub fn is_thrusting(&self) -> bool {
        self.throttle > THRUSTER_DEAD_BAND
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }
}
