use crate::factory::Recipe;
use crate::nanotime::Nanotime;

#[derive(Debug)]
pub struct Plant {
    name: String,
    recipe: Recipe,
    duration: Nanotime,
    elapsed: Nanotime,
    is_active: bool,
    inputs: Vec<u64>,
    outputs: Vec<u64>,
}

impl Plant {
    pub fn new(name: impl Into<String>, recipe: Recipe, duration: Nanotime) -> Self {
        Self {
            name: name.into(),
            recipe,
            duration,
            elapsed: Nanotime::zero(),
            is_active: false,
            inputs: Vec::new(),
            outputs: Vec::new(),
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
}
