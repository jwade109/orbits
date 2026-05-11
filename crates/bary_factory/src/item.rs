use crate::item_filter::ItemFilter;
use bary_core::prelude::*;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Sequence, Serialize, Deserialize)]
pub enum Item {
    Iron,
    Copper,
    Magnesium,
    Silicon,
    Titanium,
    Ice,
    Stone,
    Concrete,
    Plastic,
    Bread,
    /// H2O, 18 g/mol
    Water,
    /// CH4; 16 g/mol
    Methane,
    /// H2;   2 g/mol
    H2,
    /// CO2; 44 g/mol
    CO2,
    /// O2;  32 g/mol
    O2,
    Calzones,
    Geodes,
    Wheat,
    Corn,
    Milk,
    U238,
    U235,
    Rotor,
    Circuit,
    TitaniumLattice,
    PowerCell,
}

impl Item {
    pub fn to_sprite_name(&self) -> String {
        format!("item-{:?}", self).to_lowercase()
    }

    pub fn all() -> impl Iterator<Item = Self> {
        enum_iterator::all::<Self>()
    }

    pub fn all_fluids() -> impl Iterator<Item = Self> {
        Self::all().filter(|item| item.is_fluid())
    }

    pub fn all_mineable() -> impl Iterator<Item = Self> {
        Self::all().filter(|item| item.is_mineable())
    }

    pub fn random() -> Self {
        let variants: Vec<_> = Self::all().collect();
        let n = randint(0, variants.len() as i32);
        variants[n as usize]
    }

    pub fn random_mineable() -> Self {
        let variants: Vec<_> = Self::all_mineable().collect();
        let n = randint(0, variants.len() as i32);
        variants[n as usize]
    }

    pub fn random_fluid() -> Self {
        let variants: Vec<_> = Self::all_fluids().collect();
        let n = randint(0, variants.len() as i32);
        variants[n as usize]
    }

    pub fn random_with_filter(filter: &ItemFilter) -> Option<Self> {
        let variants: Vec<_> = Self::all_that_passes(filter).collect();
        if variants.is_empty() {
            None
        } else {
            let n = randint(0, variants.len() as i32);
            Some(variants[n as usize])
        }
    }

    pub fn all_that_passes<'a>(filter: &'a ItemFilter) -> impl Iterator<Item = Item> + use<'a> {
        Self::all().filter(|item| filter.passes(*item))
    }

    pub fn is_solid_cargo(&self) -> bool {
        match self {
            Item::Iron => true,
            Item::Copper => true,
            Item::Magnesium => true,
            Item::Silicon => true,
            Item::Titanium => true,
            Item::Ice => true,
            Item::Stone => true,
            Item::Concrete => true,
            Item::Plastic => true,
            Item::Bread => true,
            Item::Water => false,
            Item::Methane => false,
            Item::H2 => false,
            Item::CO2 => false,
            Item::O2 => false,
            Item::Calzones => true,
            Item::Geodes => true,
            Item::Wheat => true,
            Item::Corn => true,
            Item::Milk => false,
            Item::U235 => true,
            Item::U238 => true,
            Item::Rotor => true,
            Item::Circuit => true,
            Item::TitaniumLattice => true,
            Item::PowerCell => true,
        }
    }

    pub fn is_pellet(&self) -> bool {
        match self {
            Item::Iron => true,
            Item::Copper => true,
            Item::Magnesium => true,
            Item::Silicon => true,
            Item::Titanium => true,
            Item::Ice => true,
            Item::Stone => true,
            Item::Concrete => true,
            Item::Plastic => true,
            Item::Bread => false,
            Item::Water => false,
            Item::Methane => false,
            Item::H2 => false,
            Item::CO2 => false,
            Item::O2 => false,
            Item::Calzones => false,
            Item::Geodes => false,
            Item::Wheat => false,
            Item::Corn => false,
            Item::Milk => false,
            Item::U235 => true,
            Item::U238 => true,
            Item::Rotor => false,
            Item::Circuit => false,
            Item::TitaniumLattice => false,
            Item::PowerCell => false,
        }
    }

    pub fn is_fluid(&self) -> bool {
        match self {
            Item::Iron => false,
            Item::Copper => false,
            Item::Magnesium => false,
            Item::Silicon => false,
            Item::Titanium => false,
            Item::Ice => false,
            Item::Stone => false,
            Item::Concrete => false,
            Item::Plastic => false,
            Item::Bread => false,
            Item::Water => true,
            Item::Methane => true,
            Item::H2 => true,
            Item::CO2 => true,
            Item::O2 => true,
            Item::Calzones => false,
            Item::Geodes => false,
            Item::Wheat => false,
            Item::Corn => false,
            Item::Milk => false,
            Item::U235 => false,
            Item::U238 => false,
            Item::Rotor => false,
            Item::Circuit => false,
            Item::TitaniumLattice => false,
            Item::PowerCell => false,
        }
    }

    pub fn is_mineable(&self) -> bool {
        match self {
            Item::Iron | Item::Copper | Item::Magnesium | Item::U238 | Item::Geodes | Item::Ice => {
                true
            }
            _ => false,
        }
    }

    // most item units are grams, i.e. grams of iron ore or hydrogen gas.
    // other items, such as rotors, circuits, beams, etc have larger masses
    // per unit.
    pub fn mass_per_unit(&self) -> Mass {
        match self {
            // "continuous" items which behave as if they're fluids
            Item::Iron
            | Item::Copper
            | Item::Magnesium
            | Item::Silicon
            | Item::Titanium
            | Item::Ice
            | Item::Stone
            | Item::Concrete
            | Item::Plastic
            | Item::Water
            | Item::Methane
            | Item::H2
            | Item::CO2
            | Item::O2
            | Item::Milk
            | Item::U238
            | Item::U235 => Mass::grams(1),

            // TODO revisit these.
            Item::Corn => Mass::grams(120),
            Item::Bread => Mass::grams(200),
            Item::Wheat => Mass::grams(110),
            Item::Calzones => Mass::grams(260),
            Item::Geodes => Mass::grams(1300),
            Item::Rotor => Mass::grams(300),
            Item::Circuit => Mass::grams(90),
            Item::TitaniumLattice => Mass::grams(1900),
            Item::PowerCell => Mass::grams(930),
        }
    }

    // volume per "one" of the thing. for items like ore, fluid, etc,
    // this might be volume per 1 gram of the stuff.
    // for items like rotors, bread, etc, this is the volume of one item
    // of that type.
    pub fn volume_per_unit(&self) -> Volume {
        match self {
            Item::Iron => Volume::microliters(127),
            Item::Copper => Volume::microliters(112),
            Item::Titanium => Volume::microliters(222),
            Item::Magnesium => Volume::microliters(575),
            Item::Silicon => Volume::microliters(429),
            Item::Ice => Volume::microliters(1003),
            Item::Stone => Volume::microliters(265),
            Item::Concrete => Volume::microliters(280),
            Item::Plastic => Volume::microliters(833),
            Item::Water => Volume::microliters(1003),
            Item::Methane => Volume::microliters(2360),
            Item::H2 => Volume::microliters(13890),
            Item::CO2 => Volume::milliliters_f64(542.853),
            Item::O2 => Volume::microliters(872),
            Item::Calzones => Volume::liters(2),
            Item::Geodes => Volume::liters(3),
            Item::Wheat => Volume::milliliters(1400),
            Item::Corn => Volume::milliliters(1100),
            Item::Milk => Volume::microliters(924),
            Item::U238 => Volume::microliters(53),
            Item::U235 => Volume::microliters(53),

            // larger items
            Item::Bread => Volume::milliliters(1500),
            Item::Circuit => Volume::milliliters(410),
            Item::TitaniumLattice => Volume::milliliters(3500),
            Item::PowerCell => Volume::milliliters(1700),
            Item::Rotor => Volume::milliliters(1200),
        }
    }

    pub fn color(&self) -> [u8; 3] {
        match self {
            Item::Iron => [165, 156, 148],
            Item::Copper => [183, 119, 41],
            Item::Magnesium => [216, 216, 216],
            Item::Silicon => [149, 153, 165],
            Item::Titanium => [135, 134, 129],
            Item::Ice => [175, 238, 238],
            Item::Stone => [70, 70, 70],
            Item::Concrete => [213, 207, 207],
            Item::Plastic => [250, 249, 246],
            Item::Bread => [153, 101, 21],
            Item::Water => [0, 105, 148],
            Item::Methane => [0, 168, 107],
            Item::H2 => [255, 153, 19],
            Item::CO2 => [255, 114, 118],
            Item::O2 => [176, 224, 230],
            Item::Calzones => [153, 101, 21],
            Item::Geodes => [186, 182, 170],
            Item::Wheat => [246, 190, 0],
            Item::Corn => [247, 229, 148],
            Item::Milk => [255, 255, 255],
            Item::U235 => [66, 200, 47],
            Item::U238 => [6, 64, 43],
            Item::Rotor => [173, 0, 0],
            Item::Circuit => [17, 191, 11],
            Item::TitaniumLattice => [0, 26, 128],
            Item::PowerCell => [161, 88, 103],
        }
    }
}
