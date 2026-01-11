#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MachineStatus {
    #[default]
    Off,
    NoRecipe,
    Running,
    NoRoom,
    Starved,
}

impl MachineStatus {
    pub fn is_running(&self) -> bool {
        *self == MachineStatus::Running
    }
}
