use crate::factory::Mass;
use crate::math::*;
use crate::prelude::PHYSICS_CONSTANT_DELTA_TIME;
use serde::{Deserialize, Serialize};

/// Definition of a thruster model.
/// These are stats common to all thrusters
/// of a given type, i.e. F1, J2, LEM descent, etc
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThrusterModel {
    dims: UVec2,
    mass: Mass,
    name: String,
    pub model: String,
    thrust: f32,
    pub exhaust_velocity: f32,
    pub is_rcs: bool,
    pub throttle_rate: f32,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
    pub plume_length: f32,
    pub plume_angle: f32,
    pub minimum_throttle: f32,
}

impl ThrusterModel {
    pub fn max_thrust(&self) -> f32 {
        self.thrust
    }

    pub fn current_thrust(&self, data: &ThrusterInstanceData) -> f32 {
        if data.is_thrusting(self) {
            self.thrust * data.throttle()
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThrusterInstanceData {
    throttle: f32,
    target_throttle: f32,
    seconds_remaining: f32,
}

impl ThrusterInstanceData {
    pub fn new() -> Self {
        Self {
            throttle: 0.0,
            target_throttle: 0.0,
            seconds_remaining: 20.0,
        }
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
    }

    pub fn target_throttle(&self) -> f32 {
        self.target_throttle
    }

    pub fn set_throttle(&mut self, throttle: f32) {
        self.target_throttle = throttle.clamp(0.0, 1.0);
    }

    pub fn seconds_remaining(&self) -> f32 {
        self.seconds_remaining
    }

    pub fn on_sim_tick(&mut self, model: &ThrusterModel) {
        let dt = PHYSICS_CONSTANT_DELTA_TIME;
        let dthrottle = (model.throttle_rate * dt.to_secs()).abs();
        let diff = (self.target_throttle - self.throttle).abs();
        if self.throttle < self.target_throttle {
            self.throttle += dthrottle.min(diff);
        } else if self.throttle > self.target_throttle {
            self.throttle -= dthrottle.min(diff);
        }
        self.throttle = self.throttle.clamp(0.0, 1.0);

        self.seconds_remaining -= PHYSICS_CONSTANT_DELTA_TIME.to_secs() * self.throttle;
        if self.seconds_remaining < 0.0 {
            self.seconds_remaining = 20.0;
        }
    }

    pub fn is_thrusting(&self, model: &ThrusterModel) -> bool {
        self.throttle > model.minimum_throttle
    }
}

// TODO make this a per-thruster setting.
// deep throttling is not a given for all rocket motors
// and is in fact rather rare. KSP has spoiled us.
const _THRUSTER_DEAD_BAND: f32 = 0.0; // minimum 0 percent throttle

impl ThrusterModel {
    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn is_rcs(&self) -> bool {
        self.is_rcs
    }

    pub fn fuel_consumption_rate(&self, data: &ThrusterInstanceData) -> f32 {
        if data.is_thrusting(self) {
            let max_rate = self.thrust / self.exhaust_velocity;
            max_rate * data.throttle
        } else {
            0.0
        }
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }
}
