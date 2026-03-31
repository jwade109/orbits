use serde::{Deserialize, Serialize};

use crate::{
    factory::{Inventory, MachineStatus, Recipe, RecipeListing},
    parts::MachineData,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Machine {
    pub enabled: bool,
    pub steps: u32,
    pub required_steps: u32,
    pub recipe: RecipeListing,
    pub products_finished: u64,
    pub status: MachineStatus,
    pub input_slots: Vec<usize>,
    pub output_slots: Vec<usize>,
}

impl Machine {
    pub fn from_data(data: MachineData) -> Self {
        Self {
            enabled: true,
            recipe: RecipeListing::CarbonDioxideCondensation,
            steps: 0,
            required_steps: 1000,
            products_finished: 0,
            status: MachineStatus::Off,
            input_slots: data.input_slots,
            output_slots: data.output_slots,
        }
    }

    pub fn is_running(&self) -> bool {
        self.status == MachineStatus::Running
    }

    pub fn progress(&self) -> f32 {
        self.steps as f32 / self.required_steps as f32
    }

    pub fn set_recipe(&mut self, recipe: RecipeListing) {
        self.recipe = recipe;
        self.steps = 0;
    }

    pub fn recipe(&self) -> Option<Recipe> {
        if self.recipe == RecipeListing::DoNothing {
            None
        } else {
            Some(self.recipe.to_recipe())
        }
    }

    fn take_inputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let recipe = match self.recipe() {
            Some(r) => r,
            _ => return false,
        };

        for (i, (item, count)) in recipe.inputs().enumerate() {
            let Some(slot_index) = self.input_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot(*slot_index) else {
                return false;
            };
            if !slot.can_take(item, count) {
                return false;
            }
        }

        for (i, (item, count)) in recipe.inputs().enumerate() {
            let Some(slot_index) = self.input_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot_mut(*slot_index) else {
                return false;
            };
            if !slot.take(item, count) {
                return false;
            }
        }

        return true;
    }

    fn store_outputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let recipe = match self.recipe() {
            Some(r) => r,
            _ => return false,
        };

        for (i, (item, count)) in recipe.outputs().enumerate() {
            let Some(slot_index) = self.output_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot(*slot_index) else {
                return false;
            };
            if let Err(_) = slot.can_store(item, count) {
                return false;
            }
        }

        for (i, (item, count)) in recipe.outputs().enumerate() {
            let Some(slot_index) = self.output_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot_mut(*slot_index) else {
                return false;
            };
            if !slot.store(item, count) {
                return false;
            }
        }

        return true;
    }

    pub fn step_process(&mut self, inv: &mut Inventory) {
        if self.recipe().is_none() {
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
