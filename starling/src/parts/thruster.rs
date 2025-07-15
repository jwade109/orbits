use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

/// Definition of a thruster model.
/// These are stats common to all thrusters
/// of a given type, i.e. F1, J2, LEM descent, etc
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThrusterModel {
    dims: UVec2,
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
struct ThrusterState {
    throttle: f32,
}

impl Default for ThrusterState {
    fn default() -> Self {
        Self { throttle: 0.0 }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thruster {
    model: ThrusterModel,
    mass: Mass,
    name: String,
    #[serde(skip)]
    instance_data: ThrusterState,
}

// TODO make this a per-thruster setting.
// deep throttling is not a given for all rocket motors
// and is in fact rather rare. KSP has spoiled us.
const _THRUSTER_DEAD_BAND: f32 = 0.0; // minimum 0 percent throttle

impl Thruster {
    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.model.dims
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
            Vec2::X * self.model.thrust * self.instance_data.throttle
        } else {
            Vec2::ZERO
        }
    }

    pub fn fuel_consumption_rate(&self) -> f32 {
        if self.is_thrusting() {
            let max_rate = self.model.thrust / self.model.exhaust_velocity;
            max_rate * self.instance_data.throttle
        } else {
            0.0
        }
    }

    pub fn set_thrusting(&mut self, throttle: f32) {
        // TODO!

        self.instance_data.throttle = throttle.clamp(0.0, 1.0);

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

    pub fn is_thrusting(&self) -> bool {
        self.instance_data.throttle > 0.0
    }

    pub fn throttle(&self) -> f32 {
        self.instance_data.throttle
    }

    pub fn model_name(&self) -> &str {
        &self.model.model
    }

    pub fn model(&self) -> &ThrusterModel {
        &self.model
    }

    pub fn current_mass(&self) -> Mass {
        self.mass
    }
}
