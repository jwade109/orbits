use bary_parts::ExcavatorPrototype;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Excavator {
    pub radius: f32,
    pub ticks: u32,
    pub times_mined: u64,
}

impl Excavator {
    pub fn from_proto(ex: &ExcavatorPrototype) -> Self {
        Self {
            radius: ex.radius,
            ticks: 0,
            times_mined: 0,
        }
    }

    pub fn mined_this_tick(&self) -> bool {
        self.ticks == 0
    }

    pub fn tick(&mut self) {
        self.ticks += 1;
        if self.ticks.is_multiple_of(100) {
            self.ticks = 0;
        }
    }
}
