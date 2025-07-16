use crate::factory::Mass;
use crate::math::*;
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
pub struct ThrusterInstanceData {
    throttle: f32,
}

impl ThrusterInstanceData {
    pub fn new() -> Self {
        Self { throttle: 0.0 }
    }

    pub fn throttle(&self) -> f32 {
        self.throttle
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

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn is_rcs(&self) -> bool {
        self.is_rcs
    }

    #[deprecated]
    pub fn thrust_vector(&self, data: &ThrusterInstanceData) -> Vec2 {
        if self.is_thrusting(data) {
            Vec2::X * self.thrust * data.throttle
        } else {
            Vec2::ZERO
        }
    }

    #[deprecated]
    pub fn fuel_consumption_rate(&self, data: &ThrusterInstanceData) -> f32 {
        if self.is_thrusting(data) {
            let max_rate = self.thrust / self.exhaust_velocity;
            max_rate * data.throttle
        } else {
            0.0
        }
    }

    #[deprecated]
    pub fn set_thrusting(&self, throttle: f32, data: &mut ThrusterInstanceData) {
        // TODO!

        data.throttle = throttle.clamp(0.0, 1.0);

        // let throttle = if throttle > THRUSTER_DEAD_BAND {
        //     throttle
        // } else {
        //     0.0
        // };

        // let dt = stamp - self.instance_data.stamp;
        // self.instance_data.stamp = stamp;
        // let dthrottle = (self.model.throttle_rate * dt.to_secs()).abs();
        // let diff = (throttle - self.instance_data.throttle).abs();
        // if self.instance_data.throttle < throttle {
        //     self.instance_data.throttle += dthrottle.min(diff);
        // } else if self.instance_data.throttle > throttle {
        //     self.instance_data.throttle -= dthrottle.min(diff);
        // }
        // self.instance_data.throttle = self.instance_data.throttle.clamp(0.0, 1.0);
    }

    #[deprecated]
    pub fn is_thrusting(&self, data: &ThrusterInstanceData) -> bool {
        data.throttle > 0.0
    }

    #[deprecated]
    pub fn throttle(&self, data: &ThrusterInstanceData) -> f32 {
        data.throttle
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }
}
