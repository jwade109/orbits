use crate::starling::id::EntityId;
use crate::starling::vehicle::VehicleControl;
use std::collections::HashMap;

pub struct ControlSignals {
    pub piloting_commands: HashMap<EntityId, (VehicleControl, f32)>,
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
