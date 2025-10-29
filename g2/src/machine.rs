use std::collections::HashMap;

use crate::inventory::*;
use crate::recipe::*;
use crate::spacecraft::SpacecraftGrid;
use bevy::prelude::*;
use starling::prelude::randint;

#[derive(Component, Debug, Clone)]
pub struct Machine {
    pub enabled: bool,
    pub steps: u32,
    pub required_steps: u32,
    pub recipe: Option<Recipe>,
    pub products_finished: u64,
    pub status: MachineStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineStatus {
    Off,
    NoRecipe,
    Running,
    NoRoom,
    Starved,
}

impl Machine {
    pub fn new(recipe: impl Into<Option<Recipe>>) -> Self {
        Self {
            enabled: true,
            recipe: recipe.into(),
            steps: 0,
            required_steps: randint(20, 32) as u32,
            products_finished: 0,
            status: MachineStatus::Off,
        }
    }

    pub fn is_running(&self) -> bool {
        self.status == MachineStatus::Running
    }

    pub fn progress(&self) -> f32 {
        self.steps as f32 / self.required_steps as f32
    }

    pub fn set_recipe(&mut self, recipe: impl Into<Option<Recipe>>) {
        self.recipe = recipe.into();
        self.steps = 0;
    }

    fn take_inputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let recipe = match &self.recipe {
            Some(r) => r,
            _ => return false,
        };

        for (item, count) in recipe.inputs() {
            if !inv.can_take(item, count) {
                return false;
            }
        }

        for (item, count) in recipe.inputs() {
            if !inv.take(item, count) {
                return false;
            }
        }

        return true;
    }

    fn store_outputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let recipe = match &self.recipe {
            Some(r) => r,
            _ => return false,
        };

        for (item, count) in recipe.outputs() {
            if !inv.can_store(item, count) {
                return false;
            }
        }

        for (item, count) in recipe.outputs() {
            if !inv.store(item, count) {
                return false;
            }
        }

        return true;
    }

    pub fn step_process(&mut self, inv: &mut Inventory) {
        if self.recipe.is_none() {
            self.status = MachineStatus::NoRecipe;
            return;
        }

        if !self.enabled {
            self.status = MachineStatus::Off;
            return;
        }

        if self.steps == 0 {
            if self.take_inputs_if_possible(inv) {
                self.steps += 1;
                self.status = MachineStatus::Running;
                return;
            } else {
                self.status = MachineStatus::Starved;
                return;
            }
        }

        if self.steps > 0 && self.steps < self.required_steps {
            self.status = MachineStatus::Running;
            self.steps += 1;
        } else if self.steps >= self.required_steps {
            if self.store_outputs_if_possible(inv) {
                self.steps = 0;
                self.products_finished += 1;
                self.status = MachineStatus::Running;
            } else {
                self.status = MachineStatus::NoRoom;
            }
        } else {
            self.status = MachineStatus::Off;
        }
    }
}

pub fn update_machines(mut machines: Query<(&mut Machine, &mut Inventory)>) {
    for (mut m, mut inv) in &mut machines {
        m.step_process(&mut inv);
    }
}

#[derive(Default, Debug)]
struct BalanceManifest {
    count: u64,
    capacity: u64,
    members: Vec<(Entity, usize)>,
}

fn update_item_count(
    map: &mut HashMap<Item, BalanceManifest>,
    id: Entity,
    idx: usize,
    slot: &InvSlot,
) {
    let (item, count) = if let Some(contents) = slot.contents() {
        contents
    } else {
        return;
    };

    match slot.policy() {
        SlotPolicy::Storage => (),
        _ => return,
    };

    if let Some(bm) = map.get_mut(&item) {
        bm.count += count;
        bm.capacity += slot.capacity();
        bm.members.push((id, idx));
    } else {
        let mut bm = BalanceManifest::default();
        bm.count = count;
        bm.capacity = slot.capacity();
        bm.members.push((id, idx));
        map.insert(item, bm);
    }
}

pub fn mix_inventories(
    grids: Query<&Children, With<SpacecraftGrid>>,
    mut inventories: Query<&mut Inventory>,
) {
    for grid in grids {
        let mut manifests = HashMap::new();

        for child in grid {
            if let Ok(inv) = inventories.get(*child) {
                for (idx, slot) in inv.slots().enumerate() {
                    update_item_count(&mut manifests, *child, idx, slot);
                }
            }
        }
        manifests.retain(|_, bm| bm.count > 0 && bm.members.len() > 1);

        for (item, bm) in manifests {
            let ideal_fill_ratio = bm.count as f64 / bm.capacity as f64;

            for (entity, idx) in bm.members {
                if let Ok(mut inv) = inventories.get_mut(entity) {
                    if let Some(slot) = inv.get_slot_mut(idx) {
                        let ideal_amount =
                            (slot.capacity() as f64 * ideal_fill_ratio).round() as u64;
                        if ideal_amount > slot.count() {
                            slot.store(item, 100.min(ideal_amount - slot.count()));
                        } else if ideal_amount < slot.count() {
                            slot.take(item, 100.min(slot.count() - ideal_amount));
                        }
                    }
                }
            }
        }
    }
}
