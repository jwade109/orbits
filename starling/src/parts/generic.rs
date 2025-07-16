use crate::factory::Mass;
use crate::math::*;
use crate::parts::PartLayer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Generic {
    name: String,
    dims: UVec2,
    layer: PartLayer,
    mass: Mass,
}

impl Generic {
    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn layer(&self) -> PartLayer {
        self.layer
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }
}
