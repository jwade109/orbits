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
    sprites: Option<usize>,
}

impl Generic {
    pub fn new(name: String, dims: UVec2, layer: PartLayer, mass: Mass) -> Self {
        Self {
            name,
            dims,
            layer,
            mass,
            sprites: None,
        }
    }

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

    pub fn sprites(&self) -> usize {
        self.sprites.unwrap_or(1)
    }
}
