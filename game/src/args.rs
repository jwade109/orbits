use clap::Parser;
use std::path::PathBuf;

/// Game arguments
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct ProgramContext {
    /// Directory for game assets and saved files
    #[arg(long)]
    pub install_dir: String,
}

impl ProgramContext {
    pub fn vehicle_dir(&self) -> PathBuf {
        PathBuf::from(self.install_dir.clone()).join("vehicles")
    }

    pub fn parts_dir(&self) -> PathBuf {
        PathBuf::from(self.install_dir.clone()).join("parts")
    }
}
