use crate::constants::NOMINAL_DT;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PingParticle {
    pub pos: Vec2,
    age: f32,
}

impl PingParticle {
    pub fn new(pos: Vec2) -> Self {
        Self { pos, age: 0.0 }
    }

    pub fn step(&mut self) {
        self.age += NOMINAL_DT;
    }

    pub fn radius(&self) -> f32 {
        5.0
    }

    fn alpha(&self) -> f32 {
        1.0
    }

    pub fn is_alive(&self) -> bool {
        self.age < 4.0
    }

    pub fn is_visible(&self) -> bool {
        self.age % 1.0 < 0.5
    }
}
