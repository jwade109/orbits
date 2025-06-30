use crate::factory::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Recipe {
    inputs: HashMap<Item, u64>,
    outputs: HashMap<Item, u64>,
}

impl Recipe {
    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }

    pub fn inputs(&self) -> impl Iterator<Item = (Item, u64)> + use<'_> {
        self.inputs.iter().map(|(item, count)| (*item, *count))
    }

    pub fn outputs(&self) -> impl Iterator<Item = (Item, u64)> + use<'_> {
        self.outputs.iter().map(|(item, count)| (*item, *count))
    }

    pub fn is_input(&self, item: Item) -> bool {
        self.inputs.contains_key(&item)
    }

    pub fn is_output(&self, item: Item) -> bool {
        self.outputs.contains_key(&item)
    }
}

impl std::fmt::Display for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} => {:?}", &self.inputs, &self.outputs)
    }
}

pub fn apply_recipe(inv: &mut Inventory, recipe: &Recipe) -> bool {
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

pub fn sabatier_reaction() -> Recipe {
    Recipe {
        inputs: HashMap::from([(Item::CO2, 44), (Item::H2, 8)]),
        outputs: HashMap::from([(Item::Methane, 16), (Item::Water, 36)]),
    }
}

pub fn water_electrolysis() -> Recipe {
    Recipe {
        inputs: HashMap::from([(Item::Water, 9)]),
        outputs: HashMap::from([(Item::O2, 8), (Item::H2, 1)]),
    }
}

pub fn carbon_dioxide_condensation() -> Recipe {
    Recipe {
        inputs: HashMap::from([]),
        outputs: HashMap::from([(Item::CO2, 100)]),
    }
}

pub fn harvest_bread() -> Recipe {
    Recipe {
        inputs: HashMap::from([]),
        outputs: HashMap::from([(Item::Bread, 10)]),
    }
}

pub fn ice_melting() -> Recipe {
    Recipe {
        inputs: HashMap::from([(Item::Ice, 500)]),
        outputs: HashMap::from([(Item::Water, 500)]),
    }
}

pub fn ice_mining() -> Recipe {
    Recipe {
        inputs: HashMap::from([]),
        outputs: HashMap::from([(Item::Ice, 10)]),
    }
}

pub fn people_eat_things() -> Recipe {
    Recipe {
        inputs: HashMap::from([(Item::Water, 1_000_000), (Item::Bread, 1_000_000)]),
        outputs: HashMap::from([(Item::People, 1)]),
    }
}
