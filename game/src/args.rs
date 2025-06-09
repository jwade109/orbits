use clap::Parser;
use std::path::PathBuf;

/// Game arguments
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct ProgramContext {
    /// Directory for game assets and saved files
    #[arg(long)]
    pub install_dir: PathBuf,
}

impl ProgramContext {
    pub fn new(install_dir: PathBuf) -> Self {
        Self { install_dir }
    }

    pub fn vehicle_dir(&self) -> PathBuf {
        self.install_dir.join("vehicles")
    }

    pub fn parts_dir(&self) -> PathBuf {
        self.install_dir.join("parts")
    }

    pub fn audio_dir(&self) -> PathBuf {
        self.install_dir.join("sfx")
    }
}
