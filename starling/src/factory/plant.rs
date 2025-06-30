use crate::factory::{Item, Recipe};
use crate::nanotime::Nanotime;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Plant {
    name: String,
    recipe: Recipe,
    duration: Nanotime,
    elapsed: Nanotime,
    is_active: bool,
    inputs: HashMap<Item, u64>,
    outputs: HashMap<Item, u64>,
}

pub struct Port {}

impl Plant {
    pub fn new(name: impl Into<String>, recipe: Recipe, duration: Nanotime) -> Self {
        Self {
            name: name.into(),
            recipe,
            duration,
            elapsed: Nanotime::zero(),
            is_active: false,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn recipe(&self) -> &Recipe {
        &self.recipe
    }

    pub fn duration(&self) -> Nanotime {
        self.duration
    }

    pub fn toggle(&mut self) {
        self.is_active = !self.is_active;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn step_forward_by(&mut self, duration: Nanotime) -> u32 {
        let counts = duration.inner() / self.duration.inner();
        self.elapsed = (duration + self.elapsed) % self.duration;
        counts as u32
    }

    pub fn progress(&self) -> f32 {
        self.elapsed.to_secs() / self.duration.to_secs()
    }

    pub fn connect_input(&mut self, item: Item, id: u64) {
        if self.recipe.is_input(item) {
            self.inputs.insert(item, id);
        }
    }

    pub fn connect_output(&mut self, item: Item, id: u64) {
        if self.recipe.is_output(item) {
            self.outputs.insert(item, id);
        }
    }

    pub fn input_ports(&self) -> impl Iterator<Item = (Item, u64)> + use<'_> {
        self.inputs.iter().map(|(item, id)| (*item, *id))
    }

    pub fn output_ports(&self) -> impl Iterator<Item = (Item, u64)> + use<'_> {
        self.outputs.iter().map(|(item, id)| (*item, *id))
    }
}
