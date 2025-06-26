use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryItem {
    Iron,
    Copper,
    Magnesium,
    Silicon,
    Titanium,
    Foodstuffs,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory(HashMap<InventoryItem, u64>);

impl Inventory {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&InventoryItem, &u64)> {
        self.0.iter()
    }

    pub fn count(&self, item: InventoryItem) -> u64 {
        self.0.get(&item).cloned().unwrap_or(0)
    }

    pub fn take_all(&mut self, item: InventoryItem) -> u64 {
        self.0.remove(&item).unwrap_or(0)
    }

    pub fn has(&mut self, item: InventoryItem) -> bool {
        self.0.contains_key(&item)
    }

    pub fn take(&mut self, item: InventoryItem, count: u64) -> u64 {
        let n = self.take_all(item);
        let remaining = if count > n { 0 } else { n - count };
        if remaining > 0 {
            self.add(item, remaining);
        }
        n.min(count)
    }

    pub fn add(&mut self, item: InventoryItem, count: u64) {
        let old = self.count(item);
        self.0.insert(item.clone(), old + count);
    }
}

impl std::fmt::Display for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, (k, v)) in self.iter().enumerate() {
            write!(f, "{:?}: {} g", k, v)?;
            if i + 1 < self.len() {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory() {
        use InventoryItem::*;

        let mut inv = Inventory::new();

        assert!(inv.is_empty());

        inv.add(Copper, 45);
        inv.add(Water, 5000);
        inv.add(Copper, 400);
        inv.add(Iron, 8000);

        println!("{}", &inv);

        assert_eq!(inv.take(Copper, 50), 50);
        assert_eq!(inv.take(Copper, 800), 395);
        assert_eq!(inv.take(Iron, 600), 600);
        assert_eq!(inv.count(Iron), 7400);
        assert_eq!(inv.take(Copper, 5), 0);
        assert_eq!(inv.take_all(Iron), 7400);
        assert_eq!(inv.take_all(Iron), 0);
        assert!(inv.has(Water));
        assert!(!inv.has(Foodstuffs));
        assert!(!inv.is_empty());

        println!("{}", inv);
    }
}
