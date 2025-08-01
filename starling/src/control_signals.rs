use crate::id::EntityId;
use crate::vehicle::VehicleControl;
use std::collections::HashMap;

pub struct ControlSignals {
    pub piloting_commands: HashMap<EntityId, VehicleControl>,
}

impl ControlSignals {
    pub fn new() -> Self {
        Self {
            piloting_commands: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.piloting_commands.is_empty()
    }
}
