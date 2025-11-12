#![allow(unused)]

use crate::game_version_two::*;

#[derive(Debug, Default, Clone)]
pub struct Recipe {
    inputs: Vec<(Item, u64)>,
    outputs: Vec<(Item, u64)>,
}

impl Recipe {
    pub fn consumes(item: Item, count: u64) -> Self {
        Self {
            inputs: vec![(item, count)],
            outputs: Vec::new(),
        }
    }

    pub fn produces(item: Item, count: u64) -> Self {
        Self {
            inputs: Vec::new(),
            outputs: vec![(item, count)],
        }
    }

    pub fn and_consumes(mut self, item: Item, count: u64) -> Self {
        self.inputs.push((item, count));
        self
    }

    pub fn and_produces(mut self, item: Item, count: u64) -> Self {
        self.outputs.push((item, count));
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
        self.inputs.iter().any(|(i, _)| *i == item)
    }

    pub fn is_output(&self, item: Item) -> bool {
        self.outputs.iter().any(|(i, _)| *i == item)
    }
}

impl std::fmt::Display for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} => {:?}", &self.inputs, &self.outputs)
    }
}

pub fn sabatier_reaction() -> Recipe {
    Recipe {
        inputs: vec![(Item::CO2, 44), (Item::H2, 8)],
        outputs: vec![(Item::Methane, 16), (Item::Water, 36)],
    }
}

pub fn water_electrolysis() -> Recipe {
    Recipe {
        inputs: vec![(Item::Water, 9)],
        outputs: vec![(Item::O2, 8), (Item::H2, 1)],
    }
}

pub fn carbon_dioxide_condensation() -> Recipe {
    Recipe {
        inputs: vec![],
        outputs: vec![(Item::CO2, 100)],
    }
}

pub fn harvest_bread() -> Recipe {
    Recipe {
        inputs: vec![],
        outputs: vec![(Item::Bread, 10)],
    }
}

pub fn ice_melting() -> Recipe {
    Recipe {
        inputs: vec![(Item::Ice, 500)],
        outputs: vec![(Item::Water, 500)],
    }
}

pub fn ice_mining() -> Recipe {
    Recipe {
        inputs: vec![],
        outputs: vec![(Item::Ice, 10)],
    }
}

pub fn enrichment() -> Recipe {
    Recipe {
        inputs: vec![(Item::U238, 20), (Item::U235, 10)],
        outputs: vec![(Item::U238, 19), (Item::U235, 11)],
    }
}

pub fn titanium_lattice() -> Recipe {
    Recipe {
        inputs: vec![
            (Item::Titanium, 1400),
            (Item::Iron, 430),
            (Item::Magnesium, 70),
        ],
        outputs: vec![(Item::TitaniumLattice, 1)],
    }
}

pub fn circuits() -> Recipe {
    Recipe {
        inputs: vec![(Item::Copper, 23), (Item::Silicon, 45), (Item::Plastic, 22)],
        outputs: vec![(Item::Circuit, 1)],
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
    Enrichment,
    TitaniumLattice,
    Circuits,
}

impl RecipeListing {
    pub fn to_recipe(&self) -> Recipe {
        match self {
            RecipeListing::DoNothing => Recipe::default(),
            RecipeListing::Sabatier => sabatier_reaction(),
            RecipeListing::WaterElectrolysis => water_electrolysis(),
            RecipeListing::CarbonDioxideCondensation => carbon_dioxide_condensation(),
            RecipeListing::HarvestBread => harvest_bread(),
            RecipeListing::IceMelting => ice_melting(),
            RecipeListing::IceMining => ice_mining(),
            RecipeListing::Enrichment => enrichment(),
            RecipeListing::TitaniumLattice => titanium_lattice(),
            RecipeListing::Circuits => circuits(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_version_two::Mass;

    #[test]
    fn consistent_masses() {
        for listing in RecipeListing::all() {
            let recipe = listing.to_recipe();

            if recipe.input_count() == 0 || recipe.output_count() == 0 {
                continue;
            }

            let mut input_mass = Mass::ZERO;
            for (item, count) in recipe.inputs() {
                input_mass += item.mass_per_unit() * count;
            }
            let mut output_mass = Mass::ZERO;
            for (item, count) in recipe.outputs() {
                output_mass += item.mass_per_unit() * count;
            }
            println!("{:?}, {}, {}", listing, input_mass, output_mass);
            assert_eq!(input_mass, output_mass);
        }
    }
}
