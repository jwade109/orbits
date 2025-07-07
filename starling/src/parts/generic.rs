use crate::math::*;
use crate::parts::PartLayer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Generic {
    name: String,
    dims: UVec2,
    layer: PartLayer,
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
}
