use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum MachineStatus {
    #[default]
    Off,
    NoRecipe,
    Running,
    NoRoom,
    Starved,
    Disconnected,
    BadFilter,
}

impl MachineStatus {
    pub fn is_running(&self) -> bool {
        *self == MachineStatus::Running
    }
}
