use crate::math::randint;
use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Sequence)]
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
    People,
    Calzones,
    Geodes,
    Wheat,
    Corn,
    Milk,
    Power,
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
            Item::People => false,
            Item::Calzones => true,
            Item::Geodes => true,
            Item::Wheat => true,
            Item::Corn => true,
            Item::Milk => false,
            Item::Power => false,
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
            Item::People => false,
            Item::Calzones => false,
            Item::Geodes => false,
            Item::Wheat => false,
            Item::Corn => false,
            Item::Milk => false,
            Item::Power => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory(HashMap<Item, ItemCount>);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemCount {
    pub count: u64,
    pub capacity: u64,
}

impl Inventory {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn set_capacity(&mut self, item: Item, capacity: u64) {
        let count = self.count(item);
        let info = ItemCount {
            count: count.min(capacity),
            capacity,
        };
        self.0.insert(item, info);
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

    pub fn iter(&self) -> impl Iterator<Item = (&Item, &ItemCount)> {
        self.0.iter()
    }

    pub fn count(&self, item: Item) -> u64 {
        self.0.get(&item).map(|c| c.count).unwrap_or(0)
    }

    pub fn capacity(&self, item: Item) -> u64 {
        self.0.get(&item).map(|c| c.capacity).unwrap_or(0)
    }

    pub fn can_store(&self, item: Item, count: u64) -> bool {
        if let Some(item) = self.0.get(&item) {
            item.capacity >= item.count + count
        } else {
            false
        }
    }

    pub fn take_all(&mut self, item: Item) -> u64 {
        if let Some(info) = self.0.get_mut(&item) {
            let c = info.count;
            info.count = 0;
            c
        } else {
            0
        }
    }

    pub fn has(&mut self, item: Item) -> bool {
        self.0.get(&item).map(|c| c.count > 0).unwrap_or(false)
    }

    pub fn take(&mut self, item: Item, count: u64) -> u64 {
        let n = self.take_all(item);
        let remaining = if count > n { 0 } else { n - count };
        if remaining > 0 {
            self.add(item, remaining);
        }
        n.min(count)
    }

    pub fn add(&mut self, item: Item, count: u64) -> bool {
        if let Some(info) = self.0.get_mut(&item) {
            info.count = (info.count + count).min(info.capacity);
            return true;
        }
        return false;
    }
}

impl std::fmt::Display for Inventory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "[empty inventory]");
        }

        for (i, (k, v)) in self.iter().enumerate() {
            write!(f, "{:?}: {}/{}", k, v.count, v.capacity)?;
            if i + 1 < self.len() {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for ItemCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.count, self.capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory() {
        use Item::*;

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
        assert!(!inv.has(Bread));
        assert!(!inv.is_empty());

        println!("{}", inv);
    }
}
