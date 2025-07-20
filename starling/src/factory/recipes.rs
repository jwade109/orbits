use crate::factory::*;
use crate::math::*;
use enum_iterator::Sequence;
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Recipe {
    inputs: HashMap<Item, u64>,
    outputs: HashMap<Item, u64>,
}

impl Recipe {
    pub fn consumes(item: Item, count: u64) -> Self {
        Self {
            inputs: HashMap::from([(item, count)]),
            outputs: HashMap::new(),
        }
    }

    pub fn produces(item: Item, count: u64) -> Self {
        Self {
            inputs: HashMap::new(),
            outputs: HashMap::from([(item, count)]),
        }
    }

    pub fn and_consumes(mut self, item: Item, count: u64) -> Self {
        self.inputs.insert(item, count);
        self
    }

    pub fn and_produces(mut self, item: Item, count: u64) -> Self {
        self.outputs.insert(item, count);
        self
    }

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

#[derive(Debug, Clone, Copy, Sequence, PartialEq, Eq)]
pub enum RecipeListing {
    DoNothing, // TODO maybe don't keep this
    Sabatier,
    WaterElectrolysis,
    CarbonDioxideCondensation,
    HarvestBread,
    IceMelting,
    IceMining,
    PeopleEatThings,
}

impl RecipeListing {
    pub fn all() -> impl Iterator<Item = Self> {
        enum_iterator::all::<Self>()
    }

    pub fn random() -> Self {
        let variants: Vec<_> = Self::all().collect();
        let n = randint(0, variants.len() as i32);
        variants[n as usize]
    }
}
