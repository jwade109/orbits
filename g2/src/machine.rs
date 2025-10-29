use crate::inventory::*;
use crate::recipe::*;
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
