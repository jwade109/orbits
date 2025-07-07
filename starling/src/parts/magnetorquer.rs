use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Magnetorquer {
    dims: UVec2,
    part_name: String,
    pub max_torque: f32,
    pub current_torque: f32,
}

impl Magnetorquer {
    pub fn set_torque(&mut self, torque: f32) {
        self.current_torque = torque.clamp(-self.max_torque, self.max_torque)
    }

    pub fn part_name(&self) -> &str {
        &self.part_name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }
}
