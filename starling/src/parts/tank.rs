use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TankModel {
    pub name: String,
    pub dims: UVec2,
    pub dry_mass: Mass,
}

impl TankModel {
    pub fn part_name(&self) -> &str {
        &self.name
    }
}
