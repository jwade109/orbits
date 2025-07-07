use crate::math::{rotate, Vec2};
use crate::nanotime::Nanotime;
use serde::{Deserialize, Serialize};

/// Definition of a thruster model.
/// These are stats common to all thrusters
/// of a given type, i.e. F1, J2, LEM descent, etc
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThrusterModel {
    pub model: String,
    pub thrust: f32,
    pub exhaust_velocity: f32,
    pub length: f32,
    pub width: f32,
    pub is_rcs: bool,
    pub throttle_rate: f32,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thruster {
    model: ThrusterModel,
    pub pos: Vec2,
    pub angle: f32,
    stamp: Nanotime,
    throttle: f32,
}

// TODO make this a per-thruster setting.
// deep throttling is not a given for all rocket motors
// and is in fact rather rare. KSP has spoiled us.
const THRUSTER_DEAD_BAND: f32 = 0.0; // minimum 0 percent throttle

impl Thruster {
    pub fn new(model: ThrusterModel, pos: Vec2, angle: f32) -> Self {
        Thruster {
            model,
            pos,
            angle,
            stamp: Nanotime::zero(),
            throttle: 0.0,
        }
    }

    pub fn pointing(&self) -> Vec2 {
        rotate(Vec2::X, self.angle)
    }

    pub fn length(&self) -> f32 {
        self.model.length
    }

    pub fn width(&self) -> f32 {
        self.model.width
    }

    pub fn is_rcs(&self) -> bool {
        self.model.is_rcs
    }

    pub fn thrust_vector(&self) -> Vec2 {
        if self.is_thrusting() {
            self.pointing() * self.model.thrust * self.throttle
        } else {
            Vec2::ZERO
        }
    }

    pub fn fuel_consumption_rate(&self) -> f32 {
        if self.is_thrusting() {
            let max_rate = self.model.thrust / self.model.exhaust_velocity;
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
        let dthrottle = (self.model.throttle_rate * dt.to_secs()).abs();
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

    pub fn model_name(&self) -> &str {
        &self.model.model
    }

    pub fn model(&self) -> &ThrusterModel {
        &self.model
    }
}
