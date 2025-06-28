use crate::factory::*;
use crate::nanotime::Nanotime;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Factory {
    stamp: Nanotime,
    next_entity_id: u64,
    storage: HashMap<u64, Storage>,
    plants: HashMap<u64, Plant>,
    connections: HashSet<(u64, u64)>,
}

fn normalize_conn_indices(a: u64, b: u64) -> (u64, u64) {
    (a.min(b), a.max(b))
}

impl Factory {
    pub fn new() -> Self {
        Self {
            stamp: Nanotime::zero(),
            next_entity_id: 0,
            storage: HashMap::new(),
            plants: HashMap::new(),
            connections: HashSet::new(),
        }
    }

    pub fn stamp(&self) -> Nanotime {
        self.stamp
    }

    fn get_new_entity_id(&mut self) -> u64 {
        let ret = self.next_entity_id;
        self.next_entity_id += 1;
        ret
    }

    pub fn add_storage(&mut self, item: Item, capacity: u64) -> u64 {
        let storage = Storage::new(item, capacity);
        let id = self.get_new_entity_id();
        self.storage.insert(id, storage);
        id
    }

    pub fn add_plant(&mut self, recipe: Recipe, duration: Nanotime) -> u64 {
        let plant = Plant::new(recipe, duration);
        let id = self.get_new_entity_id();
        self.plants.insert(id, plant);
        id
    }

    pub fn connect(&mut self, a: u64, b: u64) {
        let (a, b) = normalize_conn_indices(a, b);
        self.connections.insert((a, b));
    }

    pub fn is_connected(&self, a: u64, b: u64) -> bool {
        let (a, b) = normalize_conn_indices(a, b);
        self.connections.contains(&(a, b))
    }

    pub fn plants(&self) -> impl Iterator<Item = &Plant> + use<'_> {
        self.plants.iter().map(|(_, p)| p)
    }

    pub fn storage(&self) -> impl Iterator<Item = &Storage> + use<'_> {
        self.storage.iter().map(|(_, s)| s)
    }

    pub fn storage_count(&self) -> usize {
        self.storage.len()
    }

    pub fn plant_count(&self) -> usize {
        self.plants.len()
    }

    pub fn do_stuff(&mut self, stamp: Nanotime) {
        while self.stamp < stamp {
            for (_, storage) in &mut self.storage {
                storage.add(crate::math::randint(100, 400) as u64);
            }

            self.stamp += Nanotime::mins(1);
        }
    }
}

pub fn model_factory() -> Factory {
    let mut factory = Factory::new();

    let electrolysis = water_electrolysis();

    let water = factory.add_storage(Item::Water, 1_500_000);
    let o2 = factory.add_storage(Item::O2, 3_000_000);
    let h2 = factory.add_storage(Item::H2, 700_000);
    for _ in 0..4 {
        factory.add_storage(Item::H2, 500_000);
    }

    let plant = factory.add_plant(electrolysis, Nanotime::mins(3));

    factory.connect(plant, o2);
    factory.connect(plant, h2);
    factory.connect(plant, water);

    dbg!(&factory);

    factory
}
