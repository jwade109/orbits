use crate::factory::{Item, Mass};
use crate::math::*;
use crate::parts::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Tank {
    pub item: Item,
    pub dry_mass: Mass,
    pub maximum_fuel_mass: Mass,
    pub current_fuel_mass: Mass,
}
