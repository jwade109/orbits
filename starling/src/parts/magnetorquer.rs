use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Magnetorquer {
    pub max_torque: f32,
    pub current_torque: f32,
}

impl Magnetorquer {
    pub fn set_torque(&mut self, torque: f32) {
        self.current_torque = torque.clamp(-self.max_torque, self.max_torque)
    }
}
