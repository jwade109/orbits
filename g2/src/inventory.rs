#![allow(unused)]

use crate::recipe::Recipe;
use bevy::prelude::*;
use enum_iterator::Sequence;
use starling::prelude::randint;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Sequence)]
pub enum Item {
    Iron,
    Copper,
    Magnesium,
    Silicon,
    Titanium,
    Ice,
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
}

impl Item {
    pub fn to_sprite_name(&self) -> String {
        format!("item-{:?}", self).to_lowercase()
    }

    pub fn all() -> impl Iterator<Item = Self> {
        enum_iterator::all::<Self>()
    }

    pub fn random() -> Self {
        let variants: Vec<_> = Self::all().collect();
        let n = randint(0, variants.len() as i32);
        variants[n as usize]
    }

    pub fn is_solid_cargo(&self) -> bool {
        match self {
            Item::Iron => true,
            Item::Copper => true,
            Item::Magnesium => true,
            Item::Silicon => true,
            Item::Titanium => true,
            Item::Ice => true,
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
            Item::Bread => false,
            Item::Water => false,
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
        }
    }

    // mass per unit volume, in kg/m^3
    pub fn density(&self) -> u64 {
        match self {
            Item::O2 => 1141,
            Item::H2 => 71,
            Item::Iron => 7874,
            Item::Copper => 8935,
            _ => 1000,
        }
    }

    pub fn color(&self) -> Srgba {
        let arr = match self {
            Item::Iron => [165, 156, 148],
            Item::Copper => [183, 119, 41],
            Item::Magnesium => [216, 216, 216],
            Item::Silicon => [149, 153, 165],
            Item::Titanium => [135, 134, 129],
            Item::Ice => [175, 238, 238],
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
        };
        Srgba::from_u8_array_no_alpha(arr)
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct Inventory(Vec<InvSlot>);

impl Inventory {
    pub fn zero_slots() -> Self {
        Self(Vec::new())
    }

    pub fn random_single() -> Self {
        let slot =
            InvSlot::new(SlotPolicy::Storage, 50000, ItemFilter::Any).with_item(Item::random());
        Self(vec![slot])
    }

    pub fn from_recipe(recipe: &Recipe) -> Self {
        let mut s = Self::zero_slots();

        for (item, count) in recipe.inputs() {
            let slot = InvSlot::new(SlotPolicy::Input, count * 10, ItemFilter::Any).with_item(item);
            s.0.push(slot);
        }

        for (item, count) in recipe.outputs() {
            let slot =
                InvSlot::new(SlotPolicy::Output, count * 10, ItemFilter::Any).with_item(item);
            s.0.push(slot);
        }

        s
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InvSlot> {
        self.0.iter()
    }

    pub fn can_store(&self, item: Item, count: u64) -> bool {
        // TODO this doesn't cover the case where multiple slots
        // combined can store the given amount
        self.0.iter().any(|s| s.can_store(item, count))
    }

    pub fn store(&mut self, item: Item, count: u64) -> bool {
        for slot in &mut self.0 {
            if slot.store(item, count) {
                return true;
            }
        }
        return false;
    }

    pub fn can_take(&self, item: Item, count: u64) -> bool {
        // TODO this doesn't cover the case where multiple slots
        // combined can provide the given amount
        self.0.iter().any(|s| s.can_take(item, count))
    }

    pub fn take(&mut self, item: Item, count: u64) -> bool {
        for slot in &mut self.0 {
            if slot.take(item, count) {
                return true;
            }
        }
        return false;
    }

    pub fn slots(&self) -> impl Iterator<Item = &InvSlot> + use<'_> {
        self.0.iter()
    }

    pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut InvSlot> + use<'_> {
        self.0.iter_mut()
    }

    pub fn mass(&self) -> u64 {
        self.0.iter().map(|s| s.mass()).sum()
    }

    pub fn volume(&self) -> u64 {
        self.0.iter().map(|s| s.volume()).sum()
    }
}

impl std::fmt::Display for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "[empty inventory]");
        }

        for (i, slot) in self.iter().enumerate() {
            write!(f, "{:?}", slot)?;
            if i + 1 < self.len() {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotPolicy {
    Input,
    Output,
    Storage,
}

impl SlotPolicy {
    pub fn can_take(&self) -> bool {
        match self {
            SlotPolicy::Input => false,
            SlotPolicy::Output => true,
            SlotPolicy::Storage => true,
        }
    }

    pub fn can_store(&self) -> bool {
        match self {
            SlotPolicy::Input => true,
            SlotPolicy::Output => false,
            SlotPolicy::Storage => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ItemFilter {
    Any,
    Fluids,
    Solid,
}

impl ItemFilter {
    pub fn all() -> impl Iterator<Item = ItemFilter> {
        [ItemFilter::Any, ItemFilter::Fluids, ItemFilter::Solid].into_iter()
    }

    pub fn passes(&self, item: Item) -> bool {
        match self {
            ItemFilter::Any => true,
            ItemFilter::Fluids => item.is_fluid(),
            ItemFilter::Solid => item.is_solid_cargo(),
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct InvSlot {
    policy: SlotPolicy,
    capacity: u64,
    filter: ItemFilter,
    contents: Option<(Item, u64)>,
}

impl InvSlot {
    pub fn new(policy: SlotPolicy, capacity: u64, filter: ItemFilter) -> Self {
        Self {
            policy,
            capacity,
            filter,
            contents: None,
        }
    }

    pub fn with_item(mut self, item: Item) -> Self {
        self.contents = Some((item, 0));
        self
    }

    pub fn policy(&self) -> SlotPolicy {
        self.policy
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn filter(&self) -> ItemFilter {
        self.filter
    }

    pub fn contents(&self) -> Option<(Item, u64)> {
        self.contents
    }

    pub fn set_item(&mut self, item: Item) {
        self.contents = Some((item, 0));
    }

    pub fn has(&self, item: Item) -> bool {
        self.contents.map(|(i, _)| i == item).unwrap_or(false)
    }

    pub fn add(&mut self, to_add: u64) {
        if let Some((item, count)) = self.contents {
            let new_count = (to_add + count).min(self.capacity);
            self.contents = Some((item, new_count));
        }
    }

    pub fn empty(&mut self) {
        if let Some((item, _)) = self.contents {
            self.contents = Some((item, 0));
        }
    }

    pub fn fill(&mut self) {
        if let Some((item, _)) = self.contents {
            self.contents = Some((item, self.capacity));
        }
    }

    pub fn can_take(&self, item: Item, count: u64) -> bool {
        if let Some(contents) = self.contents {
            contents.0 == item && contents.1 >= count
        } else {
            false
        }
    }

    pub fn take(&mut self, item: Item, count: u64) -> bool {
        if !self.can_take(item, count) {
            return false;
        }

        if let Some(contents) = &mut self.contents {
            contents.1 -= count;
        }

        true
    }

    // a slot can store a given amount of item IFF it has a set item,
    // and its current storage plus the new count would be less or equal to
    // the given capacity.
    // TODO related mass and volume here.
    pub fn can_store(&self, item: Item, count: u64) -> bool {
        if let Some(contents) = self.contents {
            contents.0 == item && contents.1 + count <= self.capacity
        } else {
            false
        }
    }

    pub fn store(&mut self, item: Item, count: u64) -> bool {
        if !self.can_store(item, count) {
            return false;
        }

        if let Some(contents) = &mut self.contents {
            contents.1 += count;
        }

        true
    }

    pub fn fill_percentage(&self) -> f32 {
        self.contents
            .map(|(_, c)| (c as f64 / self.capacity as f64) as f32)
            .unwrap_or(0.0)
    }

    pub fn item(&self) -> Option<Item> {
        self.contents.map(|(i, _)| i)
    }

    pub fn mass(&self) -> u64 {
        self.contents
            .map(|(item, count)| item.density() * count / 1000)
            .unwrap_or(0)
    }

    pub fn volume(&self) -> u64 {
        // TODO
        self.capacity
    }
}
