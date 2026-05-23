use crate::*;
use bary_core::prelude::*;
use bary_terminal::*;

fn parse_arg<T>(args: &ArgsMap, key: &'static str) -> Result<T, ParseError>
where
    T: std::str::FromStr,
{
    let arg = args.get(key).ok_or(ParseError::BadKey)?;
    arg.parse().map_err(|_| ParseError::BadValue)
}

pub fn cmd_ping(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Ok(TermCmd::Ping)
}

pub fn cmd_spawn(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let bp = parse_arg(args, "bp_name")?;
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    Ok(TermCmd::World(WorldAction::SpawnShipAt(
        bp,
        Isometry2d::from_pos(Vec2::new(x, y)),
    )))
}

pub fn cmd_waypoint(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let grid_id = parse_arg(args, "grid_id")?;
    let x = parse_arg(args, "x")?;
    let y = parse_arg(args, "y")?;
    let pos = Vec2::new(x, y);
    Ok(TermCmd::SetWaypoint(Ent(grid_id), Isometry2d::from_pos(pos)))
}

pub fn cmd_edit(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let _grid_id = Ent(parse_arg(args, "grid_id")?);
    Err(ParseError::NotImplemented)
}

pub fn cmd_find(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let name = parse_arg(args, "grid_name")?;
    Ok(TermCmd::FindGridByName(name))
}

pub fn cmd_despawn(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let grid_id = Ent(parse_arg(args, "grid_id")?);
    Ok(TermCmd::World(WorldAction::DespawnGrid(grid_id)))
}

pub fn cmd_set_speed(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let speed = parse_arg(args, "speed")?;
    Ok(TermCmd::SetSimSpeed(speed))
}

pub fn cmd_placeholder(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Err(ParseError::NotImplemented)
}

pub fn cmd_set_cpu(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let state = parse_arg(args, "state")?;
    Ok(TermCmd::Client(ClientAction::SetCpuSelectedGrid(state)))
}

pub fn cmd_say(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let state = parse_arg(args, "msg")?;
    Ok(TermCmd::Say(state))
}

pub fn cmd_exit(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Ok(TermCmd::Exit)
}

pub fn cmd_clear(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Ok(TermCmd::Clear)
}

pub fn server_console_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("echo", vec![], |_| Ok(TermCmd::EchoSave)),
        Command::new("load", vec!["path"], |args| {
            let save = parse_arg(args, "path")?;
            Ok(TermCmd::LoadSave(save))
        }),
        Command::new("save", vec!["savename", "overwrite"], |args| {
            let savename = parse_arg(args, "savename")?;
            let overwrite = parse_arg(args, "overwrite")?;
            Ok(TermCmd::SaveWorldToDisk(savename, overwrite))
        }),
        Command::new("clear", vec![], cmd_clear),
        Command::new("ls.saves", vec![], |_| Ok(TermCmd::ListSaves)),
        Command::new("speed", vec!["speed"], cmd_set_speed),
    ]
}

pub fn all_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("spawn", vec!["bp_name", "x", "y"], cmd_spawn),
        Command::new("edit", vec!["grid_id"], cmd_edit),
        Command::new("despawn", vec!["grid_id"], cmd_despawn),
        Command::new("find", vec!["grid_name"], cmd_find),
        Command::new("ping", vec![], cmd_ping),
        Command::new("waypoint", vec!["grid_id", "x", "y"], cmd_waypoint),
        Command::new("speed", vec!["speed"], cmd_set_speed),
        Command::new("save", vec![], cmd_placeholder),
        Command::new("exit", vec![], cmd_exit),
        Command::new("setcpu", vec!["state"], cmd_set_cpu),
        Command::new("say", vec!["msg"], cmd_say),
        Command::new("clear", vec![], cmd_clear),
        Command::new("ls.grids", vec![], |_| Ok(TermCmd::ListGrids)),
        Command::new("ls.protos", vec![], |_| Ok(TermCmd::ListProtos)),
        Command::new("ls.parts", vec![], |_| Ok(TermCmd::ListParts)),
        Command::new("ls.thrusters", vec![], |_| Ok(TermCmd::ListThrusters)),
        Command::new("ls.cpus", vec![], |_| Ok(TermCmd::ListComputers)),
        Command::new("server.disconnect", vec![], |_| {
            Ok(TermCmd::ServerDisconnect)
        }),
        Command::new("server.connect", vec![], |_| Ok(TermCmd::ServerConnect)),
        Command::new("server.info", vec![], |_| {
            Ok(TermCmd::RequestServerStatistics)
        }),
    ]
}
