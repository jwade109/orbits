#![allow(unused)]

use std::collections::HashMap;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum InventoryItem {
    Iron,
    Copper,
    Magnesium,
    Silicon,
    Titanium,
    Foodstuffs,
    Water,
    Fuel,
}

#[derive(Debug, Clone)]
pub struct Inventory(HashMap<InventoryItem, u64>);

impl Inventory {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, item: &InventoryItem) -> u64 {
        self.0.get(item).cloned().unwrap_or(0)
    }

    pub fn add(&mut self, item: InventoryItem, count: u64) {
        let old = self.get(&item);
        self.0.insert(item.clone(), old + count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory() {
        use InventoryItem::*;

        let mut inv = Inventory::new();

        inv.add(Copper, 45);
        inv.add(Fuel, 5000);

        let taken = inv.add(Copper, 400);

        dbg!(inv);
    }
}
