use crate::prelude::Vec2;
use crate::vehicle::VehicleControl;

pub struct ControlSignals {
    pub gravity: f32,
    pub piloting: Option<VehicleControl>,
}

impl ControlSignals {
    pub fn new() -> Self {
        Self {
            gravity: 5.0,
            piloting: None,
        }
    }

    pub fn gravity_vector(&self) -> Vec2 {
        -Vec2::Y * self.gravity
    }
}
