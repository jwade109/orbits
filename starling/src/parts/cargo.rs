use crate::factory::{Item, Mass};
use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Cargo {
    name: String,
    dry_mass: Mass,
    max_cargo_mass: Mass,
    dims: UVec2,
}

impl Cargo {
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

    pub fn put(&self, item: Item, mass: Mass, data: &mut CargoInstanceData) {
        if !data.has_any(item) && !data.has_empty_slot() {
            return;
        }

        if !item.is_solid_cargo() {
            return;
        }

        let contents_mass = data.contents_mass();

        if contents_mass == self.capacity_mass() {
            return;
        }

        assert!(contents_mass < self.capacity_mass());

        if contents_mass + mass > self.capacity_mass() {
            data.put(item, self.capacity_mass() - contents_mass);
        } else {
            data.put(item, mass);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CargoInstanceData {
    contents: [Option<(Item, Mass)>; 4],
}

impl CargoInstanceData {
    pub fn new() -> Self {
        CargoInstanceData {
            contents: [None, None, None, None],
        }
    }

    pub fn clear_contents(&mut self) {
        self.contents = [None, None, None, None]
    }

    pub fn contents(&self) -> impl Iterator<Item = (Item, Mass)> + use<'_> {
        self.contents.iter().filter_map(|e| {
            let (item, mass) = (*e)?;
            Some((item, mass))
        })
    }

    pub fn has_empty_slot(&self) -> bool {
        self.contents.iter().any(|e| e.is_none())
    }

    pub fn has_any(&self, item: Item) -> bool {
        self.contents
            .iter()
            .any(|e| e.map(|(e, _)| e == item).unwrap_or(false))
    }

    pub fn contents_mass(&self) -> Mass {
        self.contents
            .iter()
            .map(|contents| contents.map(|(_, mass)| mass).unwrap_or(Mass::ZERO))
            .sum()
    }

    pub fn put(&mut self, item: Item, mass: Mass) {
        if !item.is_solid_cargo() {
            return;
        }

        if self.has_any(item) {
            for slot in &mut self.contents {
                if let Some((slot, stored)) = slot {
                    if *slot == item {
                        *stored += mass;
                    }
                }
            }
        } else {
            for slot in &mut self.contents {
                if slot.is_none() {
                    *slot = Some((item, mass));
                    break;
                }
            }
        }
    }
}
