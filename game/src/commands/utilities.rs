use crate::commands::*;
use crate::game::GameState;
use clap::Parser;

#[derive(Parser, Debug, Default, Clone)]
#[command(about)]
pub struct Pwd;

impl Command for Pwd {
    fn execute(&self, state: &mut GameState) -> Result<(), String> {
        let pwd = std::env::current_dir();
        state.console.print(format!("WD: {:?}", pwd));
        let exe = std::env::current_exe();
        state.console.print(format!("EXE: {:?}", exe));
        state.console.print(format!("Args: {:?}", state.args));
        Ok(())
    }
}

#[derive(Parser, Debug, Default, Clone)]
#[command(about)]
pub struct Listing;

impl Command for Listing {
    fn execute(&self, state: &mut GameState) -> Result<(), String> {
        let line: String = enum_iterator::all::<CommandDecl>()
            .map(|e| format!("{:?} ", e))
            .collect();
        state.console.print(line);
        Ok(())
    }
}
