use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Machine {
    dims: UVec2,
    mass: Mass,
}

#[derive(Debug, Clone, Copy)]
pub struct MachineInstanceData {
    pub steps_completed: u32,
    pub steps_required: u32,
}

impl Default for MachineInstanceData {
    fn default() -> Self {
        MachineInstanceData {
            steps_completed: 0,
            steps_required: 100,
        }
    }
}

impl MachineInstanceData {
    pub fn on_sim_tick(&mut self) {
        self.steps_completed += 1;
        self.steps_completed %= self.steps_required + 1;
    }

    pub fn percent_complete(&self) -> f32 {
        self.steps_completed as f32 / self.steps_required as f32
    }
}

impl Machine {
    pub fn part_name(&self) -> &str {
        "chemical-plant"
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }
}
