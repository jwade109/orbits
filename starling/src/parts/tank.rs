use crate::factory::{Item, Mass};
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tank {
    name: String,
    dims: UVec2,
    pub item: Item,
    pub dry_mass: Mass,
    pub max_fluid_mass: Mass,

    #[serde(skip)]
    instance_data: TankState,
}

#[derive(Debug, Clone, Copy)]
struct TankState {
    current_fluid_mass: Mass,
}

impl Default for TankState {
    fn default() -> Self {
        Self {
            current_fluid_mass: Mass::kilograms(1000),
        }
    }
}

impl Tank {
    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn stored(&self) -> Mass {
        self.instance_data.current_fluid_mass
    }

    pub fn take(&mut self, mass: Mass) {
        if mass < self.instance_data.current_fluid_mass {
            self.instance_data.current_fluid_mass -= mass;
        } else {
            self.instance_data.current_fluid_mass = Mass::ZERO;
        }
    }
}
