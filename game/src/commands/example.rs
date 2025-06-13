use crate::commands::command::Command;
use crate::game::GameState;
use clap::Parser;

/// Example command arguments
#[derive(Parser, Debug, Default, Clone)]
#[command(about, long_about)]
pub struct ExampleCommand {
    /// Name of the things
    #[arg(long)]
    pub name: String,
    /// number of things
    #[arg(short, long)]
    pub count: u32,
}

impl Command for ExampleCommand {
    fn apply(&self, state: &GameState) -> Option<()> {
        println!("wow! {:?} {}", self, state.wall_time);
        None
    }
}
