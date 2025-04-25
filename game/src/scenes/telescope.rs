use crate::{scenes::Scene, ui::InteractionEvent};
use starling::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct TelescopeScene {
    pub center: Vec2,
}

impl TelescopeScene {
    pub fn new() -> Self {
        TelescopeScene { center: Vec2::ZERO }
    }

    pub fn on_interaction(&mut self, inter: &InteractionEvent) {
        self.center += randvec(1.0, 10.0);
    }
}
