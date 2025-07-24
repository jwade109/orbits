use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Settings {
    pub ui_button_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ui_button_height: 32.0,
        }
    }
}
