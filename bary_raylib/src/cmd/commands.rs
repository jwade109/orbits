use crate::multiplayer::Action;
use bary_core::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ParseError {
    BadKey,
    BadValue,
    WrongArgumentCount,
    CommandNotFound,
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
    Ok(Action::Ping(Vec2::new(x, y)))
}

pub fn cmd_spawn(args: &ArgsMap) -> Result<Action, ParseError> {
    let bp = parse_arg(args, "bp_name")?;
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    Ok(Action::SpawnShipAt(
        bp,
        Isometry2d::from_pos(Vec2::new(x, y)),
    ))
}

pub fn cmd_waypoint(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_id = parse_arg(args, "grid_id")?;
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    let pos = Vec2::new(x, y);
    Ok(Action::SetWaypoint {
        grid_id: Ent(grid_id),
        waypoint: Isometry2d::from_pos(pos),
    })
}

pub fn cmd_goto(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_name = parse_arg(args, "grid_name")?;
    Ok(Action::LookAt(grid_name))
}

pub fn cmd_edit(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_id = Ent(parse_arg(args, "grid_id")?);
    dbg!(grid_id);
    Err(ParseError::CommandNotFound)
}

pub fn cmd_find(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_id = Ent(parse_arg(args, "grid_id")?);
    dbg!(grid_id);
    Err(ParseError::CommandNotFound)
}

pub fn cmd_despawn(args: &ArgsMap) -> Result<Action, ParseError> {
    let grid_id = Ent(parse_arg(args, "grid_id")?);
    Ok(Action::DespawnGrid(grid_id))
}

pub fn cmd_placeholder(_args: &ArgsMap) -> Result<Action, ParseError> {
    Err(ParseError::CommandNotFound)
}
