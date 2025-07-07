use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Machine {
    dims: UVec2,
}

impl Machine {
    pub fn part_name(&self) -> &str {
        "chemical-plant"
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }
}
