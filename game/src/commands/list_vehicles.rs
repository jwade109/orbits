use crate::commands::command::Command;
use crate::game::GameState;
use clap::Parser;

/// Example command arguments
#[derive(Parser, Debug, Default, Clone)]
#[command(about, long_about)]
pub struct ListVehicles {}

impl Command for ListVehicles {
    fn execute(&self, state: &mut GameState) -> Result<(), String> {
        for (id, ov) in &state.universe.orbital_vehicles {
            let s = format!(
                "{:?}: orbital name=\"{}\" d={}",
                id,
                ov.vehicle.name(),
                ov.vehicle.discriminator()
            );
            state.console.print(s);
        }
        for (id, sv) in &state.universe.surface_vehicles {
            let s = format!(
                "{:?}: surface name=\"{}\" d={}",
                id,
                sv.vehicle.name(),
                sv.vehicle.discriminator()
            );
            state.console.print(s);
        }
        Ok(())
    }
}
