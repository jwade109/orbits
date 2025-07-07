use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Machine {
    dims: UVec2,
    mass: Mass,
}

impl Machine {
    pub fn part_name(&self) -> &str {
        "chemical-plant"
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn current_mass(&self) -> Mass {
        self.mass
    }
}
