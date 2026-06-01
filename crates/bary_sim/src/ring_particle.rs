use crate::TICKS_PER_SECOND;
use crate::constants::NOMINAL_DT;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PingParticle {
    pos: Vec2,
    start_tick: u64,
    duration: u64,
}

impl PingParticle {
    pub fn new(pos: Vec2, start_tick: u64) -> Self {
        Self {
            pos,
            start_tick,
            duration: TICKS_PER_SECOND * 10,
        }
    }

    pub fn start_tick(&self) -> u64 {
        self.start_tick
    }

    pub fn pos(&self) -> Vec2 {
        self.pos
    }

    pub fn radius(&self) -> f32 {
        5.0
    }

    pub fn is_alive(&self, current_tick: u64) -> bool {
        current_tick < self.start_tick + self.duration
    }
}
