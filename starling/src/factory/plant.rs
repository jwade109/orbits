use crate::factory::Recipe;
use crate::nanotime::Nanotime;

#[derive(Debug)]
pub struct Plant {
    recipe: Recipe,
    duration: Nanotime,
    process_start: Option<Nanotime>,
    is_active: bool,
}

impl Plant {
    pub fn new(recipe: Recipe, duration: Nanotime) -> Self {
        Self {
            recipe,
            duration,
            process_start: None,
            is_active: false,
        }
    }

    pub fn recipe(&self) -> &Recipe {
        &self.recipe
    }

    pub fn duration(&self) -> Nanotime {
        self.duration
    }
}
