use crate::commands::*;
use crate::game::GameState;
use clap::Parser;
use enum_iterator::*;
use std::fmt::Debug;

pub trait Command {
    fn execute(&self, state: &mut GameState) -> Result<(), String>;
}

fn do_command<T: Parser + Debug + Command>(state: &mut GameState, args: Vec<String>) {
    let args = match T::try_parse_from(args) {
        Ok(args) => args,
        Err(e) => {
            state.console.print(format!("{}", e));
            return;
        }
    };

    state.console.print(format!("{:?}", args));

    let ret = args.execute(state);

    state.console.print(format!("{:?}", ret));

    return;
}

#[derive(Sequence, Debug)]
pub enum CommandDecl {
    Example,
    Pwd,
    Listing,
}

impl CommandDecl {
    pub fn execute(&self, state: &mut GameState, args: Vec<String>) {
        match self {
            CommandDecl::Example => do_command::<Example>(state, args),
            CommandDecl::Pwd => do_command::<Pwd>(state, args),
            CommandDecl::Listing => do_command::<Listing>(state, args),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        for variant in all::<Self>() {
            let v = format!("{:?}", variant);
            if s.to_lowercase() == v.to_lowercase() {
                return Some(variant);
            }
        }
        None
    }
}
