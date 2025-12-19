use crate::factory::Mass;
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cargo {
    name: String,
    dry_mass: Mass,
    max_cargo_mass: Mass,
    dims: UVec2,
    sprites: Option<usize>,
    slots: u8,
}

impl Cargo {
    pub fn new(name: String, dry_mass: Mass, max_cargo_mass: Mass, dims: UVec2) -> Self {
        Self {
            name,
            dry_mass,
            max_cargo_mass,
            dims,
            sprites: None,
            slots: 1,
        }
    }

    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn empty_mass(&self) -> Mass {
        self.dry_mass
    }

    pub fn capacity_mass(&self) -> Mass {
        self.max_cargo_mass
    }

    pub fn sprites(&self) -> usize {
        self.sprites.unwrap_or(1)
    }

    pub fn slots(&self) -> u8 {
        self.slots
    }
}
