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

    pub fn connect_input(&mut self, plant_id: u64, item: Item, input_id: u64) -> Option<()> {
        let plant = self.plants.get_mut(&plant_id)?;
        plant.connect_input(item, input_id);
        Some(())
    }

    pub fn connect_output(&mut self, plant_id: u64, item: Item, output_id: u64) -> Option<()> {
        let plant = self.plants.get_mut(&plant_id)?;
        plant.connect_output(item, output_id);
        Some(())
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

            for (_, plant) in &mut self.plants {
                if crate::math::rand(0.0, 1.0) < 0.001 {
                    plant.toggle();
                }

                for (_, id) in plant.input_ports() {
                    if let Some(storage) = self.storage.get_mut(&id) {
                        storage.take(100);
                    }
                }

                for (_, id) in plant.output_ports() {
                    if let Some(storage) = self.storage.get_mut(&id) {
                        storage.add(100);
                    }
                }
            }

            self.stamp += dt;
        }
    }
}

pub fn model_factory() -> Factory {
    let mut factory = Factory::new();

    let ice = factory.add_storage(Item::Ice, 300_000);
    let water = factory.add_storage(Item::Water, 1_500_000);
    let o2 = factory.add_storage(Item::O2, 3_000_000);
    let h2 = factory.add_storage(Item::H2, 700_000);
    let methane = factory.add_storage(Item::Methane, 12_000_000);
    let co2 = factory.add_storage(Item::CO2, 12_000_000);

    // mines ice
    let miner = factory.add_plant("miner", ice_mining(), Nanotime::hours(1));
    factory.connect_output(miner, Item::Ice, ice);

    // melts ice
    let heater = factory.add_plant("heater", ice_melting(), Nanotime::hours(1));
    factory.connect_input(heater, Item::Ice, ice);
    factory.connect_output(heater, Item::Water, water);

    // water to o2 and h2
    let electro = factory.add_plant("electro", water_electrolysis(), Nanotime::mins(270));
    factory.connect_input(electro, Item::Water, water);
    factory.connect_output(electro, Item::O2, o2);
    factory.connect_output(electro, Item::H2, h2);

    // h2 and co2 to methane and water
    let chemplant = factory.add_plant("chemplant", sabatier_reaction(), Nanotime::days(3));
    factory.connect_input(chemplant, Item::H2, h2);
    factory.connect_input(chemplant, Item::CO2, co2);
    factory.connect_output(chemplant, Item::Water, water);
    factory.connect_output(chemplant, Item::Methane, methane);

    dbg!(&factory);

    factory
}
