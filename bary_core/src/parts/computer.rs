use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ComputerData {
    pub ticks_per_cycle: u32,
}
