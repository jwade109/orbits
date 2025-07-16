use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Radar {
    dims: UVec2,
    mass: Mass,
}

impl Radar {
    pub fn part_name(&self) -> &str {
        "radar"
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }
}
