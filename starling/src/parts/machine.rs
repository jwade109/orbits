use crate::factory::*;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Machine {
    dims: UVec2,
    mass: Mass,
    sprites: Option<usize>,
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

    pub fn sprites(&self) -> usize {
        self.sprites.unwrap_or(1)
    }
}
