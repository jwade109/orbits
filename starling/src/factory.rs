use crate::inventory::{Inventory, InventoryItem};
use crate::nanotime::Nanotime;

#[derive(Debug)]
pub struct Factory {
    stamp: Nanotime,
}

#[derive(Debug)]
pub struct Recipe {
    inputs: Vec<(InventoryItem, u64)>,
    outputs: Vec<(InventoryItem, u64)>,
}

impl Factory {
    fn new() -> Self {
        Self {
            stamp: Nanotime::zero(),
        }
    }
}

fn apply_recipe(inv: &mut Inventory, recipe: &Recipe) -> bool {
    for (item, count) in &recipe.inputs {
        if inv.count(*item) < *count {
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
        let factory = Factory::new();

        // CO2 (44) + 4H2 (8) -> CH4 (16) + 2H20 (36)

        let sabatier = Recipe {
            inputs: vec![(InventoryItem::CO2, 44), (InventoryItem::H2, 8)],
            outputs: vec![(InventoryItem::Methane, 16), (InventoryItem::Water, 36)],
        };

        let carbon_dioxide_condensation = Recipe {
            inputs: vec![],
            outputs: vec![(InventoryItem::CO2, 44)],
        };

        let electrolysis = Recipe {
            inputs: vec![(InventoryItem::Water, 9)],
            outputs: vec![(InventoryItem::O2, 8), (InventoryItem::H2, 1)],
        };

        let mut inv = Inventory::new();

        // feedstock!
        inv.add(InventoryItem::H2, 3000);

        println!("{}", &inv);

        for i in 0..2000 {
            apply_recipe(&mut inv, &carbon_dioxide_condensation);
            println!("{} {}", i, &inv);
            if !apply_recipe(&mut inv, &sabatier) {
                inv.take_all(InventoryItem::CO2);
                break;
            }
            println!("{} {}", i, &inv);
        }

        println!("{}", &inv);

        while apply_recipe(&mut inv, &electrolysis) {}

        println!("{}", &inv);
    }
}
