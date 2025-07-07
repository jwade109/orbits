use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cargo {
    name: String,
    dry_mass: Mass,
    dims: UVec2,
}

impl Cargo {
    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn current_mass(&self) -> Mass {
        // TODO needs to consider payload mass!
        self.dry_mass
    }

    pub fn dry_mass(&self) -> Mass {
        self.dry_mass
    }
}
