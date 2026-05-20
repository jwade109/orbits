use crate::*;
use bary_core::prelude::*;
use std::collections::BTreeMap;

pub struct Command {
    pub entrypoint: String,
    pub params: Vec<String>,
    pub func: Box<dyn Fn(&ArgsMap) -> Result<Action, ParseError>>,
}

impl Command {
    fn new(
        entrypoint: &'static str,
        params: Vec<&'static str>,
        f: impl Fn(&ArgsMap) -> Result<Action, ParseError> + 'static,
    ) -> Self {
        Self {
            entrypoint: entrypoint.to_string(),
            params: params.iter().map(|s| s.to_string()).collect(),
            func: Box::new(f),
        }
    }

    pub fn to_suggestion(&self) -> String {
        let mut ret = self.entrypoint.clone();
        for param in &self.params {
            ret += &format!(" [{}]", param);
        }
        ret
    }

    pub fn parse_partial_args(&self, args: &[String]) -> Vec<(Option<String>, Option<String>)> {
        let mut ret = Vec::new();

        for i in 0..self.params.len().max(args.len()) {
            let p = self.params.get(i);
            let a = args.get(i);
            ret.push((p.cloned(), a.cloned()));
        }
        ret
    }

    pub fn parse_complete_args(&self, args: &[String]) -> Option<ArgsMap> {
        let mut ret = ArgsMap::new();
        for (i, param) in self.params.iter().enumerate() {
            let arg = args.get(i)?;
            ret.insert(param.clone(), arg.clone());
        }
        Some(ret)
    }

    pub fn parse(&self, args: &[String]) -> Result<Action, ParseError> {
        let args = self
            .parse_complete_args(args)
            .ok_or(ParseError::WrongArgumentCount)?;
        (self.func)(&args)
    }
}

#[derive(Debug)]
pub enum ParseError {
    BadKey,
    BadValue,
    WrongArgumentCount,
    CommandNotFound,
    NotImplemented,
}

pub type ArgsMap = BTreeMap<String, String>;

fn parse_arg<T>(args: &ArgsMap, key: &'static str) -> Result<T, ParseError>
where
    T: std::str::FromStr,
{
    let arg = args.get(key).ok_or(ParseError::BadKey)?;
    arg.parse().map_err(|_| ParseError::BadValue)
}

pub fn cmd_ping(args: &ArgsMap) -> Result<Action, ParseError> {
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    Ok(Action::World(WorldAction::Ping(Vec2::new(x, y))))
}

pub fn cmd_spawn(args: &ArgsMap) -> Result<Action, ParseError> {
    let bp = parse_arg(args, "bp_name")?;
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    Ok(Action::World(WorldAction::SpawnShipAt(
        bp,
        Isometry2d::from_pos(Vec2::new(x, y)),
    )))
}

pub fn cmd_waypoint(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_id = parse_arg(args, "grid_id")?;
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    let pos = Vec2::new(x, y);
    Ok(Action::World(WorldAction::SetWaypoint {
        grid_id: Ent(grid_id),
        waypoint: Isometry2d::from_pos(pos),
    }))
}

pub fn cmd_edit(args: &ArgsMap) -> Result<Action, ParseError> {
    let _grid_id = Ent(parse_arg(args, "grid_id")?);
    Err(ParseError::NotImplemented)
}

pub fn cmd_find(args: &ArgsMap) -> Result<Action, ParseError> {
    let _grid_id = Ent(parse_arg(args, "grid_id")?);
    Err(ParseError::NotImplemented)
}

pub fn cmd_despawn(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_id = Ent(parse_arg(args, "grid_id")?);
    Ok(Action::World(WorldAction::DespawnGrid(grid_id)))
}

pub fn cmd_set_speed(args: &ArgsMap) -> Result<Action, ParseError> {
    let speed = parse_arg(args, "speed")?;
    Ok(Action::World(WorldAction::SetSpeed(speed)))
}

pub fn cmd_placeholder(_args: &ArgsMap) -> Result<Action, ParseError> {
    Err(ParseError::NotImplemented)
}

pub fn cmd_set_cpu(args: &ArgsMap) -> Result<Action, ParseError> {
    let state = parse_arg(args, "state")?;
    Ok(Action::Client(ClientAction::SetCpuSelectedGrid(state)))
}

pub fn cmd_say(args: &ArgsMap) -> Result<Action, ParseError> {
    let state = parse_arg(args, "msg")?;
    Ok(Action::Client(ClientAction::Say(state)))
}

pub fn all_commands() -> Vec<Command> {
    vec![
        Command::new("spawn", vec!["bp_name", "x", "y"], cmd_spawn),
        Command::new("edit", vec!["grid_id"], cmd_edit),
        Command::new("despawn", vec!["grid_id"], cmd_despawn),
        Command::new("find", vec!["grid_name"], cmd_find),
        Command::new("ping", vec!["x", "y"], cmd_ping),
        Command::new("waypoint", vec!["grid_id", "x", "y"], cmd_waypoint),
        Command::new("speed", vec!["speed"], cmd_set_speed),
        Command::new("save", vec![], cmd_placeholder),
        Command::new("exit", vec![], |_args| panic!()),
        Command::new("setcpu", vec!["state"], cmd_set_cpu),
        Command::new("say", vec!["msg"], cmd_say),
    ]
}
