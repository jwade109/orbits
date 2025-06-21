use crate::math::{rotate, Vec2};
use crate::nanotime::Nanotime;
use crate::parts::parts::ThrusterProto;

#[derive(Debug, Clone)]
pub struct Thruster {
    pub proto: ThrusterProto,
    pub pos: Vec2,
    pub angle: f32,
    pointing: Vec2,
    stamp: Nanotime,
    throttle: f32,
}

const THRUSTER_DEAD_BAND: f32 = 0.10; // minimum 10 percent throttle

impl Thruster {
    pub fn new(proto: ThrusterProto, pos: Vec2, angle: f32) -> Self {
        Thruster {
            proto,
            pos,
            angle,
            pointing: rotate(Vec2::X, angle),
            stamp: Nanotime::zero(),
            throttle: 0.0,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        self.pointing
    }

    pub fn thrust_vector(&self) -> Vec2 {
        if self.is_thrusting() {
            self.pointing * self.proto.thrust * self.throttle
        } else {
            Vec2::ZERO
        }
    }

    pub fn fuel_consumption_rate(&self) -> f32 {
        if self.is_thrusting() {
            let max_rate = self.proto.thrust / self.proto.exhaust_velocity;
            max_rate * self.throttle
        } else {
            0.0
        }
    }

    pub fn set_thrusting(&mut self, throttle: f32, stamp: Nanotime) {
        let throttle = if throttle > THRUSTER_DEAD_BAND {
            throttle
        } else {
            0.0
        };

        let dt = stamp - self.stamp;
        self.stamp = stamp;
        let dthrottle = (self.proto.throttle_rate * dt.to_secs()).abs();
        let diff = (throttle - self.throttle).abs();
        if self.throttle < throttle {
            self.throttle += dthrottle.min(diff);
        } else if self.throttle > throttle {
            self.throttle -= dthrottle.min(diff);
        }
        self.throttle = self.throttle.clamp(0.0, 1.0);
    }

    pub fn is_thrusting(&self) -> bool {
        self.throttle > 0.0
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }
}
