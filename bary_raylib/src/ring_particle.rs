use bary_core::prelude::*;

#[derive(Debug)]
pub struct RingParticle {
    pub pos: Vec2,
    pub time_left: f32,
}

impl RingParticle {
    pub fn radius(&self) -> f32 {
        self.time_left * 10.0
    }
}
