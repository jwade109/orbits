use crate::commands::command::Command;
use crate::game::GameState;
use clap::Parser;

/// Example command arguments
#[derive(Parser, Debug, Default, Clone)]
#[command(about, long_about)]
pub struct ListVehicles {}

impl Command for ListVehicles {
    fn execute(&self, state: &mut GameState) -> Result<(), String> {
        for (id, (_, _, vehicle)) in &state.universe.orbital_vehicles {
            let s = format!(
                "{:?}: orbital name=\"{}\" d={}",
                id,
                vehicle.name(),
                vehicle.discriminator()
            );
            state.console.print(s);
        }
        for (id, (_, _, vehicle)) in &state.universe.surface_vehicles {
            let s = format!(
                "{:?}: surface name=\"{}\" d={}",
                id,
                vehicle.name(),
                vehicle.discriminator()
            );
            state.console.print(s);
        }
        Ok(())
    }
}
