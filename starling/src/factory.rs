use crate::inventory::{Inventory, InventoryItem};
use crate::nanotime::Nanotime;

#[derive(Debug)]
pub struct Factory {
    pub stamp: Nanotime,
    pub inventory: Inventory,
    pub recipes: Vec<Recipe>,
}

#[derive(Debug)]
pub struct Recipe {
    inputs: Vec<(InventoryItem, u64)>,
    outputs: Vec<(InventoryItem, u64)>,
}

impl std::fmt::Display for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} => {:?}", &self.inputs, &self.outputs)
    }
}

pub fn sabatier_reaction() -> Recipe {
    Recipe {
        inputs: vec![(InventoryItem::CO2, 44), (InventoryItem::H2, 8)],
        outputs: vec![(InventoryItem::Methane, 16), (InventoryItem::Water, 36)],
    }
}

pub fn water_electrolysis() -> Recipe {
    Recipe {
        inputs: vec![(InventoryItem::Water, 9)],
        outputs: vec![(InventoryItem::O2, 8), (InventoryItem::H2, 1)],
    }
}

pub fn carbon_dioxide_condensation() -> Recipe {
    Recipe {
        inputs: vec![],
        outputs: vec![(InventoryItem::CO2, 100)],
    }
}

pub fn harvest_bread() -> Recipe {
    Recipe {
        inputs: vec![],
        outputs: vec![(InventoryItem::Bread, 10)],
    }
}

pub fn ice_melting() -> Recipe {
    Recipe {
        inputs: vec![(InventoryItem::Ice, 500)],
        outputs: vec![(InventoryItem::Water, 500)],
    }
}

pub fn ice_mining() -> Recipe {
    Recipe {
        inputs: vec![],
        outputs: vec![(InventoryItem::Ice, 10)],
    }
}

pub fn people_eat_things() -> Recipe {
    Recipe {
        inputs: vec![
            (InventoryItem::Water, 1_000_000),
            (InventoryItem::Bread, 1_000_000),
        ],
        outputs: vec![],
    }
}

impl Factory {
    pub fn new(stamp: Nanotime) -> Self {
        let mut inventory = Inventory::new();

        inventory.set_capacity(InventoryItem::CO2, 5_000_000);
        inventory.set_capacity(InventoryItem::H2, 5_000_000);
        inventory.set_capacity(InventoryItem::Methane, 5_000_000);
        inventory.set_capacity(InventoryItem::Water, 3_000_000);
        inventory.set_capacity(InventoryItem::Bread, 2_000_000);
        inventory.set_capacity(InventoryItem::Ice, 3_000_000);

        inventory.add(InventoryItem::H2, 1_000_000);

        dbg!(&inventory);

        let recipes = vec![
            sabatier_reaction(),
            carbon_dioxide_condensation(),
            harvest_bread(),
            ice_melting(),
            people_eat_things(),
            ice_mining(),
        ];

        Self {
            stamp,
            inventory,
            recipes,
        }
    }

    pub fn do_stuff(&mut self, stamp: Nanotime) {
        while self.stamp < stamp {
            self.stamp += Nanotime::mins(1);

            for recipe in &self.recipes {
                apply_recipe(&mut self.inventory, recipe);
            }
        }
    }
}

fn apply_recipe(inv: &mut Inventory, recipe: &Recipe) -> bool {
    for (item, count) in &recipe.inputs {
        if inv.count(*item) < *count {
            return false;
        }
    }

    for (item, count) in &recipe.outputs {
        if !inv.can_store(*item, *count) {
            return false;
        }
    }

    for (item, count) in &recipe.inputs {
        inv.take(*item, *count);
    }

    for (item, count) in &recipe.outputs {
        inv.add(*item, *count);
    }

    return true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_factory() {
        // OK. how simulate sabatier reaction?
        let mut inv = Inventory::new();

        // feedstock!
        inv.add(InventoryItem::H2, 3000);

        println!("{}", &inv);

        for i in 0..2000 {
            apply_recipe(&mut inv, &carbon_dioxide_condensation());
            println!("{} {}", i, &inv);
            if !apply_recipe(&mut inv, &sabatier_reaction()) {
                inv.take_all(InventoryItem::CO2);
                break;
            }
            println!("{} {}", i, &inv);
        }

        println!("{}", &inv);

        while apply_recipe(&mut inv, &water_electrolysis()) {}

        println!("{}", &inv);
    }
}
