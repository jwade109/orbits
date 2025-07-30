use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Settings {
    pub ui_button_height: f32,
    pub controller_cursor_speed: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ui_button_height: 32.0,
            controller_cursor_speed: 6.0,
        }
    }
}

pub fn load_settings_from_file(filename: &Path) -> Result<Settings, Box<dyn Error>> {
    let s = std::fs::read_to_string(filename)?;
    Ok(serde_yaml::from_str(&s)?)
}

pub fn write_settings_to_file(path: &Path, settings: &Settings) -> Result<(), Box<dyn Error>> {
    let s = serde_yaml::to_string(settings)?;
    Ok(std::fs::write(path, s)?)
}
