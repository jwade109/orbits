use crate::factory::*;
use crate::nanotime::Nanotime;
use std::collections::{HashMap, HashSet};

pub enum FactoryEntity<'a> {
    Plant(&'a Plant),
    Storage(&'a Storage),
}

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

    pub fn add_plant(
        &mut self,
        name: impl Into<String>,
        recipe: Recipe,
        duration: Nanotime,
    ) -> u64 {
        let plant = Plant::new(name, recipe, duration);
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

    pub fn plants(&self) -> impl Iterator<Item = (u64, &Plant)> + use<'_> {
        self.plants.iter().map(|(e, p)| (*e, p))
    }

    pub fn storage(&self) -> impl Iterator<Item = (u64, &Storage)> + use<'_> {
        self.storage.iter().map(|(e, s)| (*e, s))
    }

    pub fn storage_count(&self) -> usize {
        self.storage.len()
    }

    pub fn plant_count(&self) -> usize {
        self.plants.len()
    }

    pub fn do_stuff(&mut self, stamp: Nanotime) {
        while self.stamp < stamp {
            let dt = Nanotime::mins(1);

            for (_, storage) in &mut self.storage {
                let delta = crate::math::randint(-200, 400);
                if delta > 0 {
                    storage.add(delta as u64);
                } else {
                    storage.take((-delta) as u64);
                }
            }

            for (_, plant) in &mut self.plants {
                if crate::math::rand(0.0, 1.0) < 0.005 {
                    plant.toggle();
                }

                plant.step_forward_by(dt);
            }

            self.stamp += dt;
        }
    }
}

pub fn model_factory() -> Factory {
    let mut factory = Factory::new();

    let water = factory.add_storage(Item::Water, 1_500_000);
    let o2 = factory.add_storage(Item::O2, 3_000_000);
    let h2 = factory.add_storage(Item::H2, 700_000);
    for _ in 0..4 {
        factory.add_storage(Item::H2, 500_000);
    }

    let p1 = factory.add_plant("electro", water_electrolysis(), Nanotime::mins(270));

    let p2 = factory.add_plant("miner", ice_mining(), Nanotime::hours(1));
    let _ = factory.add_plant("chemplant", sabatier_reaction(), Nanotime::days(3));

    factory.connect(p1, o2);
    factory.connect(p1, h2);
    factory.connect(p1, water);

    factory.connect(p2, water);

    dbg!(&factory);

    factory
}
