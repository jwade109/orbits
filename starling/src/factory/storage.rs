use crate::factory::*;

#[derive(Debug, Clone, Copy)]
pub struct Storage {
    item: Item,
    count: u64,
    capacity: u64,
}

impl Storage {
    pub fn new(item: Item, capacity: u64) -> Self {
        Self {
            item,
            count: 0,
            capacity,
        }
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    pub fn item(&self) -> Item {
        self.item
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn clear(&mut self) {
        self.count = 0;
    }

    pub fn fill(&mut self) {
        self.count = self.capacity
    }

    pub fn fill_percent(&self) -> f32 {
        self.count as f32 / self.capacity as f32
    }

    pub fn can_store(&self, to_add: u64) -> bool {
        self.count + to_add <= self.capacity
    }

    pub fn add(&mut self, to_add: u64) {
        self.count = (self.count + to_add).min(self.capacity)
    }

    pub fn take(&mut self, to_take: u64) -> u64 {
        if to_take <= self.count {
            self.count -= to_take;
            to_take
        } else {
            let c = self.count;
            self.count = 0;
            c
        }
    }
}
