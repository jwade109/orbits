use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ComputerPrototype {
    pub ticks_per_cycle: u32,
}
