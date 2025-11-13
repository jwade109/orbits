#![allow(unused)]

use crate::game_version_two::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Sequence)]
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
            | Item::Corn
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

    pub fn color(&self) -> Srgba {
        let arr = match self {
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

    pub fn from_slots(slots: Vec<InvSlot>) -> Self {
        Self(slots)
    }

    pub fn single(item: Item, capacity: Volume) -> Self {
        let mut slot = InvSlot::new(capacity, ItemFilter::Any).with_item(item);
        Self(vec![slot])
    }

    pub fn from_recipe(recipe: &Recipe) -> Self {
        let mut s = Self::zero_slots();

        for (item, count) in recipe.inputs() {
            let capacity = item.volume_per_unit() * count * 100;
            let slot = InvSlot::new(capacity, ItemFilter::Any);
            s.0.push(slot.with_item(item));
        }

        for (item, count) in recipe.outputs() {
            let capacity = item.volume_per_unit() * count * 100;
            let slot = InvSlot::new(capacity, ItemFilter::Any);
            s.0.push(slot.with_item(item));
        }

        s
    }

    pub fn add_slot(&mut self, slot: InvSlot) {
        self.0.push(slot);
    }

    pub fn get_slot(&mut self, idx: usize) -> Option<&InvSlot> {
        self.0.get(idx)
    }

    pub fn get_slot_mut(&mut self, idx: usize) -> Option<&mut InvSlot> {
        self.0.get_mut(idx)
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

    // fills all available space with whatever the slots are set to
    pub fn fill(&mut self) {
        for slot in &mut self.0 {
            slot.fill();
        }
    }

    // Fills all available space in this inventory with the given item.
    // Returns how much was added.
    pub fn fill_with(&mut self, item: Item) -> u64 {
        let mut count = 0;
        for slot in &mut self.0.iter_mut() {
            count += slot.fill_with(item);
        }
        count
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

    pub fn slot_count(&self) -> usize {
        self.0.len()
    }

    pub fn slots(&self) -> impl Iterator<Item = &InvSlot> + use<'_> {
        self.0.iter()
    }

    pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut InvSlot> + use<'_> {
        self.0.iter_mut()
    }

    pub fn mass(&self) -> Mass {
        self.0.iter().map(|s| s.mass()).sum()
    }

    pub fn mass_of(&self, item: Item) -> Mass {
        self.0
            .iter()
            .filter_map(|s| (s.has(item)).then(|| s.mass()))
            .sum()
    }

    pub fn capacity(&self) -> Volume {
        self.0.iter().map(|s| s.capacity()).sum()
    }

    pub fn occupied_volume(&self) -> Volume {
        self.0.iter().map(|s| s.occupied_volume()).sum()
    }

    pub fn available_volume(&self) -> Volume {
        self.capacity() - self.occupied_volume()
    }

    pub fn occupied_volume_of(&self, item: Item) -> Volume {
        self.0
            .iter()
            .filter_map(|s| (s.has(item)).then(|| s.occupied_volume()))
            .sum()
    }

    pub fn available_volume_of(&self, item: Item) -> Volume {
        self.0
            .iter()
            .filter_map(|s| (s.has(item)).then(|| s.available_volume()))
            .sum()
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
    capacity: Volume,
    filter: ItemFilter,
    contents: Option<(Item, u64)>,
}

impl InvSlot {
    pub fn new(capacity: Volume, filter: ItemFilter) -> Self {
        Self {
            capacity,
            filter,
            contents: None,
        }
    }

    pub fn with_item(mut self, item: Item) -> Self {
        self.contents = Some((item, 0));
        self
    }

    pub fn capacity(&self) -> Volume {
        self.capacity
    }

    pub fn count(&self) -> u64 {
        self.contents.map(|(_, count)| count).unwrap_or(0)
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

    pub fn empty(&mut self) {
        if let Some((item, _)) = self.contents {
            self.contents = Some((item, 0));
        }
    }

    // fills this slot with the slot's set item, if possible.
    // returns the amount that was added.
    pub fn fill(&mut self) -> u64 {
        if let Some((item, count)) = self.contents {
            let units_capacity = (self.capacity / item.volume_per_unit()).floor() as u64;
            let added = units_capacity - count;
            self.contents = Some((item, units_capacity));
            added
        } else {
            0
        }
    }

    pub fn fill_with(&mut self, item: Item) -> u64 {
        if let Some((contents, _)) = self.contents {
            if contents == item { self.fill() } else { 0 }
        } else {
            0
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
        let units_capacity = (self.capacity / item.volume_per_unit()).floor() as u64;
        if let Some(contents) = self.contents {
            contents.0 == item && contents.1 + count <= units_capacity
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

    pub fn store_partial(&mut self, item: Item, count: u64) {
        let units_capacity = (self.capacity / item.volume_per_unit()).floor() as u64;
        if let Some(contents) = &mut self.contents {
            contents.1 = (contents.1 + count).min(units_capacity);
        }
    }

    pub fn fill_percentage(&self) -> f32 {
        self.contents
            .map(|(item, c)| {
                let v = item.volume_per_unit() * c;
                (v / self.capacity) as f32
            })
            .unwrap_or(0.0)
    }

    pub fn is_full(&self) -> bool {
        self.contents
            .map(|(item, _)| item.volume_per_unit() > self.available_volume())
            .unwrap_or(false)
    }

    pub fn item(&self) -> Option<Item> {
        self.contents.map(|(i, _)| i)
    }

    pub fn mass(&self) -> Mass {
        self.contents
            .map(|(item, count)| item.mass_per_unit() * count)
            .unwrap_or(Mass::ZERO)
    }

    pub fn occupied_volume(&self) -> Volume {
        self.contents
            .map(|(item, count)| count * item.volume_per_unit())
            .unwrap_or(Volume::ZERO)
    }

    pub fn available_volume(&self) -> Volume {
        self.capacity - self.occupied_volume()
    }
}

impl std::fmt::Display for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "[empty inventory]");
        }

        write!(
            f,
            "{} slots, {} / {}\n",
            self.0.len(),
            self.occupied_volume(),
            self.capacity()
        );
        for slot in &self.0 {
            let item = slot
                .contents()
                .map(|(item, count)| format!("{:?} x{}", item, count))
                .unwrap_or("(Empty)".to_string());
            let full = if slot.is_full() {
                " (full)"
            } else {
                " (not full)"
            };
            write!(
                f,
                " - {}, {}, {}/{}{}\n",
                item,
                slot.mass(),
                slot.occupied_volume(),
                slot.capacity(),
                full
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_single() {
        let mut inv = Inventory::single(Item::Iron, Volume::liters(4000));

        assert!(inv.can_store(Item::Iron, 4000));
        assert!(inv.can_store(Item::Iron, 400));
        assert!(inv.can_store(Item::Iron, 2300));
        assert!(inv.can_store(Item::Iron, 0));

        assert!(!inv.can_store(Item::Iron, 5000000000000));
        assert!(!inv.can_store(Item::Copper, 0));
        assert!(!inv.can_store(Item::Magnesium, 5000000000000));

        assert!(inv.store(Item::Iron, 1000));
        assert!(inv.store(Item::Iron, 500));

        assert!(inv.can_store(Item::Iron, 10000));
        assert!(!inv.can_store(Item::Iron, 400000000000));

        assert_eq!(inv.mass(), Mass::grams(1500));
        assert_eq!(inv.occupied_volume(), Volume::microliters(190500));

        println!("{}", inv);
    }

    #[test]
    fn multiple_slots() {
        let mut inv = Inventory::from_slots(vec![
            InvSlot::new(Volume::liters(1000), ItemFilter::Any).with_item(Item::Bread),
            InvSlot::new(Volume::liters(2000), ItemFilter::Any).with_item(Item::Magnesium),
            InvSlot::new(Volume::liters(1500), ItemFilter::Any).with_item(Item::Magnesium),
            InvSlot::new(Volume::liters(300), ItemFilter::Any).with_item(Item::Ice),
        ]);

        assert_eq!(inv.capacity(), Volume::liters(4800));
        assert_eq!(inv.occupied_volume(), Volume::ZERO);
        assert_eq!(inv.available_volume(), Volume::liters(4800));

        assert_eq!(inv.available_volume_of(Item::Bread), Volume::liters(1000));
        assert_eq!(
            inv.available_volume_of(Item::Magnesium),
            Volume::liters(3500)
        );
        assert_eq!(inv.available_volume_of(Item::Ice), Volume::liters(300));

        assert_eq!(inv.mass(), Mass::ZERO);
        assert_eq!(inv.mass_of(Item::Bread), Mass::ZERO);
        assert_eq!(inv.mass_of(Item::Magnesium), Mass::ZERO);
        assert_eq!(inv.mass_of(Item::Ice), Mass::ZERO);

        assert!(inv.store(Item::Bread, 100));
        assert!(inv.store(Item::Magnesium, 1500));

        assert_eq!(inv.capacity(), Volume::liters(4800));
        assert_eq!(inv.occupied_volume(), Volume::milliliters(800));
        assert_eq!(inv.available_volume(), Volume::microliters(4799200000));

        assert_eq!(inv.mass(), Mass::grams(21500));
        assert_eq!(inv.mass_of(Item::Bread), Mass::grams(20000));
        assert_eq!(inv.mass_of(Item::Magnesium), Mass::grams(1500));
        assert_eq!(inv.mass_of(Item::Ice), Mass::ZERO);

        println!("{}", inv);
    }

    #[test]
    fn big_items() {
        let mut inv = Inventory::from_slots(vec![
            InvSlot::new(Volume::liters(100), ItemFilter::Any).with_item(Item::Rotor),
            InvSlot::new(Volume::liters(100), ItemFilter::Any).with_item(Item::TitaniumLattice),
            InvSlot::new(Volume::liters(100), ItemFilter::Any).with_item(Item::H2),
        ]);

        while inv.store(Item::Rotor, 1) {}
        while inv.store(Item::TitaniumLattice, 1) {}
        while inv.store(Item::H2, 1) {}

        assert_eq!(inv.mass_of(Item::Rotor), Mass::grams(57000));
        assert_eq!(inv.mass_of(Item::TitaniumLattice), Mass::grams(15120));
        assert_eq!(inv.mass_of(Item::H2), Mass::grams(200000));

        println!("{}", inv);
    }
}
