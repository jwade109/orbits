use serde::{Deserialize, Serialize};

/// Definition of a thruster model.
/// These are stats common to all thrusters
/// of a given type, i.e. F1, J2, LEM descent, etc
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThrusterModel {
    pub model: String,
    thrust: f64,
    pub exhaust_velocity: f32,
    pub is_rcs: bool,
    pub throttle_rate: f32,
    pub primary_color: [f32; 4],
    pub secondary_color: [f32; 4],
    pub plume_length: f32,
    pub plume_angle: f32,
    pub minimum_throttle: f32,
    pub particle_scale: f32,
}

impl ThrusterModel {
    pub fn main_thruster(thrust: f64, ve: f32) -> Self {
        Self {
            model: "".into(),
            thrust,
            exhaust_velocity: ve,
            is_rcs: false,
            throttle_rate: 3.0,
            primary_color: [1.0, 0.3, 0.3, 1.0],
            secondary_color: [1.0, 1.0, 0.2, 1.0],
            plume_angle: 0.2,
            plume_length: 5.0,
            minimum_throttle: 0.2,
            particle_scale: 1.0,
        }
    }
}

// TODO make this a per-thruster setting.
// deep throttling is not a given for all rocket motors
// and is in fact rather rare. KSP has spoiled us.
const _THRUSTER_DEAD_BAND: f32 = 0.0; // minimum 0 percent throttle
