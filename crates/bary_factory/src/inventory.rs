#![deny(missing_docs)]

use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{Item, ItemFilter, MachineStatus};

/// An ordered list of 0 or more inventory slots. See [InvSlot]
/// for more information about semantics.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Inventory(Vec<InvSlot>);

impl Inventory {
    /// An inventory containing zero slots, which can store nothing.
    pub fn zero_slots() -> Self {
        Self(Vec::new())
    }

    /// Constructs a new Inventory with the given slots.
    pub fn from_slots(slots: Vec<InvSlot>) -> Self {
        Self(slots)
    }

    /// Constructs a new Inventory with a single slot containing
    /// none of the given [Item] and some [Volume].
    pub fn single(item: Item, capacity: Volume) -> Self {
        let slot = InvSlot::new(
            capacity,
            ItemFilter::Any,
            false,
            (PartCoord::ZERO, PartCoord::ZERO),
        )
        .with_item(item);
        Self(vec![slot])
    }

    /// Clear the contents off all slots.
    pub fn clear(&mut self) {
        self.0.iter_mut().for_each(|s| s.empty());
    }

    /// Gets a reference to the slot at the given index, if it exists.
    pub fn get_slot(&self, idx: usize) -> Option<&InvSlot> {
        self.0.get(idx)
    }

    /// Get a mutable reference to the slot at the given index, if it exists.
    pub fn get_slot_mut(&mut self, idx: usize) -> Option<&mut InvSlot> {
        self.0.get_mut(idx)
    }

    /// Get an iterator over the slots in this inventory.
    pub fn iter(&self) -> impl Iterator<Item = &InvSlot> {
        self.0.iter()
    }

    /// Get the slot at the given coordinate.
    pub fn get_slot_at(&self, p: PartCoord) -> Option<usize> {
        for (i, slot) in self.slots().enumerate() {
            let offset = (p - slot.location.0).inner();
            let max = slot.location.1.inner() - slot.location.0.inner();
            if offset.x < max.x && offset.y < max.y && offset.x >= 0 && offset.y >= 0 {
                return Some(i);
            }
        }
        None
    }

    /// Returns true if any individual slot can store the given [Item] and
    /// quantity.
    /// 
    /// TODO(feature) this should sum over all of the
    /// slots in the inventory and see if their available space together
    /// can store the requested amount.
    pub fn can_store(&self, item: Item, count: u64) -> bool {
        // TODO this doesn't cover the case where multiple slots
        // combined can store the given amount
        self.0.iter().any(|s| s.can_store(item, count).is_ok())
    }

    /// Stores the given [Item] and quantity, if possible.
    /// Returns true if successful. If not successful,
    /// the [Inventory] is not modified.
    pub fn store(&mut self, item: Item, count: u64) -> bool {
        for slot in &mut self.0 {
            if slot.store(item, count) {
                return true;
            }
        }
        return false;
    }

    /// Fills all slots with whatever item the slot is set to.
    /// If a slot doesn't have an item (even if it's empty),
    /// nothing is done to that slot.
    pub fn fill(&mut self) {
        for slot in &mut self.0 {
            slot.fill();
        }
    }

    /// Checks to see if this inventory can provide the
    /// given [Item] and quantity.
    /// 
    /// TODO(feature) currently only returns true if a single
    /// slot can provide the given amount. This function should
    /// sum over all slots to see if this inventory can provide
    /// the amount from multiple slots.
    pub fn can_take(&self, item: Item, count: u64) -> bool {
        // TODO this doesn't cover the case where multiple slots
        // combined can provide the given amount
        self.0.iter().any(|s| s.can_take(item, count))
    }

    /// Remove the given quantity of [Item], if possible.
    /// Returns true if successful. If not successful,
    /// this inventory is not modified.
    pub fn take(&mut self, item: Item, count: u64) -> bool {
        for slot in &mut self.0 {
            if slot.take(item, count) {
                return true;
            }
        }
        return false;
    }

    /// Gets the number of slots in this inventory.
    pub fn slot_count(&self) -> usize {
        self.0.len()
    }

    /// Get an iterator over the slots in this inventory.
    pub fn slots(&self) -> impl Iterator<Item = &InvSlot> + use<'_> {
        self.0.iter()
    }

    /// Get a mutable iterator over the slots in this inventory.
    pub fn slots_mut(&mut self) -> impl Iterator<Item = &mut InvSlot> + use<'_> {
        self.0.iter_mut()
    }

    /// Sum up the mass of items in this inventory.
    pub fn mass(&self) -> Mass {
        self.0.iter().map(|s| s.mass()).sum()
    }

    /// Sums the mass of all slots which contain the given item.
    pub fn mass_of(&self, item: Item) -> Mass {
        self.0
            .iter()
            .filter_map(|s| (s.has(item)).then(|| s.mass()))
            .sum()
    }

    /// Gets the total capacity of the inventory over all slots.
    pub fn capacity(&self) -> Volume {
        self.0.iter().map(|s| s.capacity()).sum()
    }

    /// Gets the sum of occupied volume over all slots.
    pub fn occupied_volume(&self) -> Volume {
        self.0.iter().map(|s| s.occupied_volume()).sum()
    }

    /// Gets the total available volume over all slots.
    pub fn available_volume(&self) -> Volume {
        self.capacity() - self.occupied_volume()
    }

    /// Gets the total volume which is occupied by the given item.
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InvSlot {
    name: Option<String>,
    capacity: Volume,
    filter: ItemFilter,
    contents: Option<(Item, u64)>,
    is_fluid: bool,
    location: (PartCoord, PartCoord),
}

impl InvSlot {
    pub fn new(
        capacity: Volume,
        filter: ItemFilter,
        is_fluid: bool,
        location: (PartCoord, PartCoord),
    ) -> Self {
        Self {
            name: None,
            capacity,
            filter,
            contents: None,
            is_fluid,
            location,
        }
    }

    pub fn location(&self) -> (PartCoord, PartCoord) {
        self.location
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
    }

    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
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

    pub fn filter(&self) -> &ItemFilter {
        &self.filter
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
        self.contents = None;
    }

    pub fn is_empty(&self) -> bool {
        self.contents.is_none()
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

    pub fn fill_with(&mut self, item: Item) {
        let units_capacity = (self.capacity / item.volume_per_unit()).floor() as u64;
        self.contents = Some((item, units_capacity));
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
            if contents.1 == 0 {
                self.contents = None;
            }
        }

        true
    }

    // a slot can store a given amount of item IFF
    // its current storage plus the new count would be less or equal to
    // the given capacity, and its filter accepts the item.
    // TODO related mass and volume here.
    pub fn can_store(&self, item: Item, count: u64) -> Result<(), MachineStatus> {
        if !self.filter.passes(item) {
            return Err(MachineStatus::BadFilter);
        }

        let units_capacity: u64 = (self.capacity / item.volume_per_unit()).floor() as u64;
        if let Some(contents) = self.contents {
            let ok = contents.0 == item && contents.1 + count <= units_capacity;
            if ok {
                Ok(())
            } else {
                Err(MachineStatus::NoRoom)
            }
        } else {
            Ok(())
        }
    }

    pub fn store(&mut self, item: Item, count: u64) -> bool {
        if !self.can_store(item, count).is_ok() {
            return false;
        }

        if let Some(contents) = &mut self.contents {
            contents.1 += count;
        } else {
            // TODO this is a bug. what if `count` is greater than capacity?
            // will we allow partial stores?
            self.contents = Some((item, count));
        }

        true
    }

    pub fn store_partial(&mut self, item: Item, count: u64) {
        let units_capacity = (self.capacity / item.volume_per_unit()).floor() as u64;
        if let Some(contents) = &mut self.contents {
            contents.1 = (contents.1 + count).min(units_capacity);
        } else {
            self.contents = Some((item, count.min(units_capacity)));
        }
    }

    pub fn fill_percentage(&self) -> f32 {
        self.contents
            .map(|(item, c)| {
                let cap = (self.capacity / item.volume_per_unit()).floor() as u64;
                (c as f64 / cap as f64) as f32
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

    pub fn is_fluid(&self) -> bool {
        self.is_fluid
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
        )?;

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

/// moves item from one inventory to another without leaving either inventory
/// in a state which would destroy or duplicate items
pub fn atomic_transfer(src: &mut InvSlot, dst: &mut InvSlot, mass: Mass) -> MachineStatus {
    if mass.is_zero() {
        return MachineStatus::Running;
    }

    let Some(item) = src.item() else {
        return MachineStatus::Starved;
    };

    if let Some(dst_item) = dst.item() {
        if dst_item != item {
            return MachineStatus::NoRoom;
        }
    }

    let src_count = src.count();

    let dst_available = (dst.available_volume() / item.volume_per_unit()).floor() as u64;

    let count = ((mass / item.mass_per_unit()).round() as u64).max(1);

    let count = count.min(src_count.min(dst_available));

    if !src.can_take(item, count) {
        return MachineStatus::Starved;
    }

    match dst.can_store(item, count) {
        Ok(()) => (),
        Err(status) => {
            return status;
        }
    }

    src.take(item, count);
    dst.store(item, count);

    return MachineStatus::Running;
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
        let loc = (PartCoord::ZERO, PartCoord::ZERO);

        let mut inv = Inventory::from_slots(vec![
            InvSlot::new(Volume::liters(1000), ItemFilter::Any, false, loc).with_item(Item::Bread),
            InvSlot::new(Volume::liters(2000), ItemFilter::Any, false, loc)
                .with_item(Item::Magnesium),
            InvSlot::new(Volume::liters(1500), ItemFilter::Any, false, loc)
                .with_item(Item::Magnesium),
            InvSlot::new(Volume::liters(300), ItemFilter::Any, false, loc).with_item(Item::Ice),
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
        assert_eq!(inv.occupied_volume(), Volume::microliters(150862500));
        assert_eq!(inv.available_volume(), Volume::microliters(4649137500));

        assert_eq!(inv.mass(), Mass::grams(21500));
        assert_eq!(inv.mass_of(Item::Bread), Mass::grams(20000));
        assert_eq!(inv.mass_of(Item::Magnesium), Mass::grams(1500));
        assert_eq!(inv.mass_of(Item::Ice), Mass::ZERO);

        println!("{}", inv);
    }

    #[test]
    fn big_items() {
        let loc = (PartCoord::ZERO, PartCoord::ZERO);

        let mut inv = Inventory::from_slots(vec![
            InvSlot::new(Volume::liters(100), ItemFilter::Any, false, loc).with_item(Item::Rotor),
            InvSlot::new(Volume::liters(100), ItemFilter::Any, false, loc)
                .with_item(Item::TitaniumLattice),
            InvSlot::new(Volume::liters(100), ItemFilter::Any, false, loc).with_item(Item::H2),
        ]);

        while inv.store(Item::Rotor, 1) {}
        while inv.store(Item::TitaniumLattice, 1) {}
        while inv.store(Item::H2, 1) {}

        assert_eq!(inv.mass_of(Item::Rotor), Mass::grams(24900));
        assert_eq!(inv.mass_of(Item::TitaniumLattice), Mass::grams(53200));
        assert_eq!(inv.mass_of(Item::H2), Mass::grams(7199));

        println!("{}", inv);
    }
}
