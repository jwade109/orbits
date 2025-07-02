use crate::factory::{Item, Recipe};
use crate::nanotime::Nanotime;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Plant {
    name: String,
    recipe: Recipe,
    duration: Nanotime,
    elapsed: Option<Nanotime>,
    is_enabled: bool,
    is_blocked: bool,
    is_starved: bool,
    input_ports: HashMap<Item, Port>,
    output_ports: HashMap<Item, Port>,
}

#[derive(Debug, Clone, Copy)]
pub struct Port {
    item: Item,
    count: u64,
    connected_to: Option<u64>,
}

impl Port {
    pub fn item(&self) -> Item {
        self.item
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn connected_to(&self) -> Option<u64> {
        self.connected_to
    }
}

impl Plant {
    pub fn new(name: impl Into<String>, recipe: Recipe, duration: Nanotime) -> Self {
        let input_ports = recipe
            .inputs()
            .map(|(item, count)| {
                (
                    item,
                    Port {
                        item,
                        count,
                        connected_to: None,
                    },
                )
            })
            .collect();

        let output_ports = recipe
            .outputs()
            .map(|(item, count)| {
                (
                    item,
                    Port {
                        item,
                        count,
                        connected_to: None,
                    },
                )
            })
            .collect();

        Self {
            name: name.into(),
            recipe,
            duration,
            elapsed: None,
            is_enabled: true,
            is_blocked: false,
            is_starved: false,
            input_ports,
            output_ports,
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
        self.is_enabled = !self.is_enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.is_enabled
    }

    pub fn is_blocked(&self) -> bool {
        self.is_blocked
    }

    pub fn is_starved(&self) -> bool {
        self.is_starved
    }

    pub fn set_blocked(&mut self, blocked: bool) {
        self.is_blocked = blocked;
    }

    pub fn set_starved(&mut self, starved: bool) {
        self.is_starved = starved;
    }

    pub fn clear_flags(&mut self) {
        self.is_blocked = false;
        self.is_starved = false;
    }

    pub fn duration_to_finish(&self) -> Option<Nanotime> {
        self.elapsed.map(|e| self.duration - e)
    }

    pub fn is_working(&self) -> bool {
        self.elapsed.is_some()
    }

    pub fn start_job(&mut self) {
        self.elapsed = Some(Nanotime::zero());
    }

    pub fn finish_job(&mut self) {
        self.elapsed = None;
    }

    pub fn step_forward_by(&mut self, duration: Nanotime) {
        if let Some(elapsed) = &mut self.elapsed {
            *elapsed = *elapsed + duration;
            if *elapsed > self.duration {
                *elapsed = self.duration;
            }
        }
    }

    pub fn progress(&self) -> f32 {
        self.elapsed
            .map(|e| e.to_secs() / self.duration.to_secs())
            .unwrap_or(0.0)
    }

    pub fn connect_input(&mut self, item: Item, id: u64) {
        if let Some(port) = self.input_ports.get_mut(&item) {
            port.connected_to = Some(id);
        }
    }

    pub fn connect_output(&mut self, item: Item, id: u64) {
        if let Some(port) = self.output_ports.get_mut(&item) {
            port.connected_to = Some(id);
        }
    }

    pub fn input_ports(&self) -> impl Iterator<Item = Port> + use<'_> {
        self.input_ports.iter().map(|(_, port)| *port)
    }

    pub fn output_ports(&self) -> impl Iterator<Item = Port> + use<'_> {
        self.output_ports.iter().map(|(_, port)| *port)
    }
}
