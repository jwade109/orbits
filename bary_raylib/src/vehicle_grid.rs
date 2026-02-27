use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VehicleGrid {
    pub name: String,
    pub parts_mass: Mass,
    pub isometry: Isometry2d,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
    pub external_thrust: IVec2,
    pub parts: Vec<Ent>,
    pub thrusters: Vec<Ent>,
    pub computers: Vec<Ent>,
    pub lights: Vec<Ent>,
    pub requires_thruster_update: bool,
}

impl VehicleGrid {
    pub fn with_name(name: impl Into<String>) -> Self {
        VehicleGrid {
            name: name.into(),
            parts_mass: Mass::ZERO,
            linear_velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            external_thrust: IVec2::ZERO,
            isometry: Isometry2d::default(),
            parts: Vec::new(),
            thrusters: Vec::new(),
            computers: Vec::new(),
            lights: Vec::new(),
            requires_thruster_update: false,
        }
    }

    pub fn linear_acceleration(&self) -> Vec2 {
        (self.external_thrust.as_vec2() / 1000.0) / (self.parts_mass.to_kg_f64() as f32)
    }
}
