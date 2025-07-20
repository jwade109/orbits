use crate::factory::{Item, Mass};
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TankModel {
    name: String,
    dims: UVec2,
    pub dry_mass: Mass,
    pub max_fluid_mass: Mass,
}

impl TankModel {
    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    // #[deprecated]
    // pub fn take(&self, mass: Mass, data: &mut TankInstanceData) {
    //     if mass < data.current_fluid_mass {
    //         data.current_fluid_mass -= mass;
    //     } else {
    //         data.current_fluid_mass = Mass::ZERO;
    //     }
    // }

    pub fn put(&self, item: Item, mass: Mass, data: &mut TankInstanceData) {
        if !item.is_fluid() {
            return;
        }

        let current_item = data.item();
        if !current_item.is_none() && current_item != Some(item) {
            return;
        }

        let mut storage = data.stored.unwrap_or((item, Mass::ZERO));

        storage.1 += mass;
        if storage.1 > self.max_fluid_mass {
            storage.1 = self.max_fluid_mass;
        }

        data.stored = Some(storage);
    }

    pub fn dry_mass(&self) -> Mass {
        self.dry_mass
    }

    pub fn capacity(&self) -> Mass {
        self.max_fluid_mass
    }

    pub fn percent_filled(&self, data: &TankInstanceData) -> f32 {
        data.contents_mass().to_kg_f32() / self.max_fluid_mass.to_kg_f32()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TankInstanceData {
    stored: Option<(Item, Mass)>,
}

impl Default for TankInstanceData {
    fn default() -> Self {
        Self { stored: None }
    }
}

impl TankInstanceData {
    pub fn contents_mass(&self) -> Mass {
        self.stored.map(|(_, mass)| mass).unwrap_or(Mass::ZERO)
    }

    pub fn item(&self) -> Option<Item> {
        self.stored.map(|(item, _)| item)
    }

    pub fn clear_contents(&mut self) {
        self.stored = None;
    }
}
