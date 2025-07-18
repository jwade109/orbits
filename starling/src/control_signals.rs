use crate::vehicle::VehicleControl;

pub struct ControlSignals {
    pub gravity: f32,
    pub piloting: Option<VehicleControl>,
    pub toggle_mode: bool,
}

impl ControlSignals {
    pub fn new() -> Self {
        Self {
            gravity: 5.0,
            piloting: None,
            toggle_mode: false,
        }
    }
}
