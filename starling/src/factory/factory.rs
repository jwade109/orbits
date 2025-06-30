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

#[derive(Debug, Clone, Copy)]
enum PlantTransitionEvent {
    StartJob,
    FinishJob,
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

    pub fn connect_input(&mut self, plant_id: u64, input_id: u64) -> Option<()> {
        let plant = self.plants.get_mut(&plant_id)?;
        let storage = self.storage.get(&input_id)?;
        let item = storage.item();
        plant.connect_input(item, input_id);
        Some(())
    }

    pub fn connect_output(&mut self, plant_id: u64, output_id: u64) -> Option<()> {
        let plant = self.plants.get_mut(&plant_id)?;
        let storage = self.storage.get(&output_id)?;
        let item = storage.item();
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

    pub fn is_plant_blocked(&self, id: u64) -> bool {
        let plant = match self.plants.get(&id) {
            Some(p) => p,
            None => return false,
        };

        for port in plant.output_ports() {
            if let Some(id) = port.connected_to() {
                if let Some(storage) = self.storage.get(&id) {
                    if !storage.can_store(port.count()) {
                        return true;
                    }
                }
            }
        }

        return false;
    }

    pub fn is_plant_starved(&self, id: u64) -> bool {
        let plant = match self.plants.get(&id) {
            Some(p) => p,
            None => return false,
        };

        for port in plant.input_ports() {
            if let Some(id) = port.connected_to() {
                if let Some(storage) = self.storage.get(&id) {
                    if !storage.has_at_least(port.count()) {
                        return true;
                    }
                }
            }
        }

        return false;
    }

    pub fn get_next_relevant_plant(&self) -> Option<(Nanotime, u64, PlantTransitionEvent)> {
        let mut results = Vec::new();
        for (id, plant) in &self.plants {
            if !plant.is_working() && !plant.is_starved() {
                results.push((self.stamp, *id, PlantTransitionEvent::StartJob));
            } else if plant.is_working() && !plant.is_blocked() {
                if let Some(dur) = plant.duration_to_finish() {
                    results.push((self.stamp + dur, *id, PlantTransitionEvent::FinishJob));
                }
            }
        }

        results.sort_by_key(|(t, _, _)| *t);

        results.get(0).cloned()
    }

    pub fn step_forward_until(&mut self, stamp: Nanotime) {
        for (_, plant) in &mut self.plants {
            plant.step_forward_by(stamp - self.stamp);
        }
        self.stamp = stamp;
    }

    pub fn do_stuff(&mut self, stamp: Nanotime) {
        while let Some((t, id, event)) = self.get_next_relevant_plant() {
            if t > stamp {
                self.step_forward_until(stamp);
                break;
            }

            let plant = match self.plants.get_mut(&id) {
                Some(p) => p,
                None => break,
            };

            match event {
                PlantTransitionEvent::StartJob => {
                    for port in plant.input_ports() {
                        if let Some(id) = port.connected_to() {
                            if let Some(storage) = self.storage.get_mut(&id) {
                                storage.take(port.count());
                            }
                        }
                    }
                    plant.start_job();
                }
                PlantTransitionEvent::FinishJob => {
                    for port in plant.output_ports() {
                        if let Some(id) = port.connected_to() {
                            if let Some(storage) = self.storage.get_mut(&id) {
                                storage.add(port.count());
                            }
                        }
                    }
                    plant.finish_job();
                }
            }

            self.step_forward_until(t);
        }

        // for (_, plant) in &mut self.plants {

        //     if plant

        //     for _ in 0..count {

        //         let mut starved = false;
        //         let mut blocked = false;

        //         for port in plant.input_ports() {
        //             if let Some(id) = port.connected_to() {
        //                 if let Some(storage) = self.storage.get(&id) {
        //                     if !storage.has_at_least(port.count()) {
        //                         starved = true;
        //                         break;
        //                     }
        //                 }
        //             }
        //         }

        //         for port in plant.output_ports() {
        //             if let Some(id) = port.connected_to() {
        //                 if let Some(storage) = self.storage.get(&id) {
        //                     if !storage.can_store(port.count()) {
        //                         blocked = true;
        //                         break;
        //                     }
        //                 }
        //             }
        //         }

        //         if starved {
        //             plant.mark_starved();
        //         }

        //         if blocked {
        //             plant.mark_blocked();
        //         }

        //         if starved || blocked {
        //             continue;
        //         }

        //         plant.clear_flags();
        //     }
        // }

        self.stamp = stamp;
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
    let miner = factory.add_plant("miner", ice_mining(), Nanotime::secs(3));
    factory.connect_output(miner, ice);

    // melts ice
    let heater = factory.add_plant("heater", ice_melting(), Nanotime::hours(1));
    factory.connect_input(heater, ice);
    factory.connect_output(heater, water);

    // water to o2 and h2
    let electro = factory.add_plant("electro", water_electrolysis(), Nanotime::mins(270));
    factory.connect_input(electro, water);
    factory.connect_output(electro, o2);
    factory.connect_output(electro, h2);

    // h2 and co2 to methane and water
    let chemplant = factory.add_plant("chemplant", sabatier_reaction(), Nanotime::days(3));
    factory.connect_input(chemplant, h2);
    factory.connect_input(chemplant, co2);
    factory.connect_output(chemplant, water);
    factory.connect_output(chemplant, methane);

    let condenser = factory.add_plant("cond", carbon_dioxide_condensation(), Nanotime::hours(1));
    factory.connect_output(condenser, co2);

    dbg!(&factory);

    factory
}

pub fn simple_factory() -> Factory {
    let mut factory = Factory::new();

    let recipe = Recipe::default()
        .produces(Item::Water, 1)
        .produces(Item::CO2, 2);
    let water = factory.add_storage(Item::Water, 1000);
    let co2 = factory.add_storage(Item::CO2, 2000);
    let plant = factory.add_plant("faucet", recipe, Nanotime::secs(1));

    factory.connect_output(plant, water);
    factory.connect_output(plant, co2);

    factory
}

pub fn example_factory() -> Factory {
    model_factory()
}
