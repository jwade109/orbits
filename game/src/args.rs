use clap::Parser;

/// Game arguments
#[derive(Parser, Debug, Default)]
#[command(version, about, long_about = None)]
pub struct ProgramArgs {
    /// Directory for game assets and saved files
    #[arg(long)]
    pub install_dir: String,
}
