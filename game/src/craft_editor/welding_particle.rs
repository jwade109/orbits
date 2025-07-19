use starling::prelude::*;

#[derive(Debug)]
pub struct BuildParticle {
    pv: PV,
    opacity: f32,
}

impl BuildParticle {
    pub fn new(pos: Vec2) -> Self {
        let vel = randvec(30.0, 90.0);
        Self {
            pv: PV::from_f64(pos, vel),
            opacity: 1.0,
        }
    }

    pub fn on_sim_tick(&mut self) {
        self.opacity -= 0.2;
        self.pv.pos += self.pv.vel * PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
    }

    pub fn pos(&self) -> Vec2 {
        self.pv.pos_f32()
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }
}
