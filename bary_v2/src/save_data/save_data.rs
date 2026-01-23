use bevy::prelude::Vec2;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::path::Path;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct ShipPosition {
    pub name: String,
    pub pos: Vec2,
    pub angle: f32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SaveData {
    pub ships: Vec<ShipPosition>,
}

impl SaveData {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let s = std::fs::read_to_string(path)?;
        let settings: Self = serde_yaml::from_str(&s)?;
        Ok(settings)
    }
}
