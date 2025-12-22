use crate::starling::units::Mass;
use crate::starling::math::*;
use serde::{Deserialize, Serialize};
use bevy::math::UVec2;

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
