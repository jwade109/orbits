use starling::prelude::*;

#[derive(Debug)]
pub struct BuildParticle {
    pv: PV,
    opacity: f32,
}

impl BuildParticle {
    pub fn new(pos: DVec2) -> Self {
        let vel = randvec(1.5, 6.0);
        Self {
            pv: PV::from_f64(pos, vel),
            opacity: 1.0,
        }
    }

    pub fn on_sim_tick(&mut self) {
        self.opacity -= 0.2;
        self.pv.pos += self.pv.vel * PHYSICS_CONSTANT_DELTA_TIME.to_secs_f64();
    }

    pub fn pos(&self) -> DVec2 {
        self.pv.pos
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }
}
