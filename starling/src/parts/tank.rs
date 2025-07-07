use crate::factory::{Item, Mass};
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Tank {
    name: String,
    dims: UVec2,
    pub dry_mass: Mass,
    pub max_fluid_mass: Mass,

    #[serde(skip)]
    instance_data: TankState,
}

#[derive(Debug, Clone, Copy)]
struct TankState {
    item: Item,
    current_fluid_mass: Mass,
}

impl Default for TankState {
    fn default() -> Self {
        println!("New tank state data");
        Self {
            item: if rand(0.0, 1.0) < 0.5 {
                Item::H2
            } else {
                Item::O2
            },
            current_fluid_mass: Mass::ZERO,
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

    pub fn put(&mut self, mass: Mass) {
        self.instance_data.current_fluid_mass += mass;
        if self.instance_data.current_fluid_mass > self.max_fluid_mass {
            self.instance_data.current_fluid_mass = self.max_fluid_mass;
        }
    }

    pub fn item(&self) -> Item {
        self.instance_data.item
    }

    pub fn current_mass(&self) -> Mass {
        self.dry_mass + self.instance_data.current_fluid_mass
    }

    pub fn dry_mass(&self) -> Mass {
        self.dry_mass
    }

    pub fn capacity(&self) -> Mass {
        self.max_fluid_mass
    }
}
