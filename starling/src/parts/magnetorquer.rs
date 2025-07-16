use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Magnetorquer {
    dims: UVec2,
    part_name: String,
    max_torque: f32,
    mass: Mass,
}

#[derive(Debug, Clone)]
pub struct MagnetorquerInstanceData {
    current_torque: f32,
}

impl Magnetorquer {
    pub fn part_name(&self) -> &str {
        &self.part_name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }
}

impl MagnetorquerInstanceData {
    pub fn new() -> Self {
        Self {
            current_torque: 0.0,
        }
    }

    pub fn torque(&self) -> f32 {
        self.current_torque
    }

    pub fn set_torque(&mut self, model: &Magnetorquer, torque: f32) {
        self.current_torque = torque.clamp(-model.max_torque, model.max_torque)
    }
}
