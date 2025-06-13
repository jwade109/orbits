use crate::commands::command::Command;
use crate::game::GameState;
use clap::Parser;
use starling::prelude::rand;

/// Example command arguments
#[derive(Parser, Debug, Default, Clone)]
#[command(about, long_about)]
pub struct Example {
    /// Name of the things
    #[arg(long)]
    pub name: String,
    /// number of things
    #[arg(short, long)]
    pub count: u32,
}

impl Command for Example {
    fn execute(&self, state: &mut GameState) -> Result<(), String> {
        if rand(0.0, 1.0) < 0.4 {
            Ok(())
        } else {
            Err(format!("oh no! {}", state.wall_time))
        }
    }
}
