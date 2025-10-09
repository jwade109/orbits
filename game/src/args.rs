use clap::Parser;
use std::path::PathBuf;
use bevy::prelude::*;

/// Game arguments
#[derive(Resource)]
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct ProgramContext {
    /// Directory for game assets and saved files
    #[arg(long)]
    assets_dir: Option<PathBuf>,
}

impl ProgramContext {
    pub fn new(assets_dir: Option<PathBuf>) -> Self {
        Self { assets_dir }
    }

    pub fn assets_dir(&self) -> PathBuf {
        let current_dir = std::env::current_dir().unwrap_or(PathBuf::new());
        self.assets_dir.clone().unwrap_or(current_dir.join("assets"))
    }

    pub fn settings_path(&self) -> PathBuf {
        self.assets_dir().join("settings.yaml")
    }

    pub fn names_path(&self) -> PathBuf {
        self.assets_dir().join("ship_names.txt")
    }

    pub fn vehicle_dir(&self) -> PathBuf {
        self.assets_dir().join("vehicles")
    }

    pub fn parts_dir(&self) -> PathBuf {
        self.assets_dir().join("parts")
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.assets_dir().join("sfx")
    }

    pub fn tutorial_path(&self) -> PathBuf {
        self.assets_dir().join("tutorial.yaml")
    }

    pub fn fonts_dir(&self) -> PathBuf {
        self.assets_dir().join("fonts")
    }

    pub fn part_sprite_path(&self, short_path: &str) -> String {
        self.parts_dir()
            .join(format!("{}/skin.png", short_path))
            .to_str()
            .unwrap_or("")
            .to_string()
    }
}
