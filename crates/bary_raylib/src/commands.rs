use crate::*;
use bary_core::prelude::*;
use bary_sim::*;
use bary_terminal::*;

fn parse_arg<T>(args: &ArgsMap, key: &'static str) -> Result<T, ParseError>
where
    T: std::str::FromStr,
{
    let arg = args.get(key).ok_or(ParseError::BadKey)?;
    arg.parse().map_err(|_| ParseError::BadValue)
}

pub fn cmd_find(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let name = parse_arg(args, "grid_name")?;
    Ok(TermCmd::FindGridByName(name))
}

pub fn cmd_set_speed(args: &ArgsMap) -> Result<TermCmd, ParseError> {
    let speed = parse_arg(args, "speed")?;
    Ok(TermCmd::SetSimSpeed(speed))
}

pub fn cmd_placeholder(_args: &ArgsMap) -> Result<TermCmd, ParseError> {
    Err(ParseError::NotImplemented)
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

pub fn client_request_blob_commands() -> Vec<Command<TermCmd>> {
    TableIdent::all()
        .map(|t| {
            let name = format!("client.req.blob.{t}").to_lowercase();
            Command::new(&name, vec![], move |_| Ok(TermCmd::ClientReqBlob(t)))
        })
        .chain([Command::new("client.req.blob.all", vec![], |_| {
            Ok(TermCmd::ClientReqAllBlobs)
        })])
        .collect()
}

pub fn blob_info_commands() -> Vec<Command<TermCmd>> {
    TableIdent::all()
        .map(|t| {
            let name = format!("print.blob.{t}").to_lowercase();
            Command::new(&name, vec![], move |_| Ok(TermCmd::PrintBlobInfo(t)))
        })
        .collect()
}

pub fn load_command() -> Command<TermCmd> {
    Command::new("load", vec!["path"], |args| {
        let save = parse_arg(args, "path")?;
        Ok(TermCmd::LoadSave(save))
    })
}

pub fn save_command() -> Command<TermCmd> {
    Command::new("save", vec!["savename", "overwrite"], |args| {
        let savename = parse_arg(args, "savename")?;
        let overwrite = parse_arg(args, "overwrite")?;
        Ok(TermCmd::SaveWorldToDisk(savename, overwrite))
    })
}

pub fn server_console_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("echo", vec![], |_| Ok(TermCmd::PrintSaveInfo)),
        load_command(),
        save_command(),
        Command::new("clear", vec![], cmd_clear),
        Command::new("speed", vec!["speed"], cmd_set_speed),
        Command::new("ls.saves", vec![], |_| Ok(TermCmd::ListSaves)),
        Command::new("ls.blueprints", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Blueprints))
        }),
        Command::new("ls.grids", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Grids))
        }),
        Command::new("ls.protos", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Protos))
        }),
        Command::new("ls.parts", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Parts))
        }),
        Command::new("ls.thrusters", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Thrusters))
        }),
        Command::new("ls.cpus", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Computers))
        }),
    ]
}

pub fn world_delta_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("sim.spawn", vec!["bp_name", "x", "y"], |args| {
            let bp = parse_arg(args, "bp_name")?;
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::SpawnShipAt(
                bp,
                Isometry2d::from_pos(Vec2::new(x, y)),
            )))
        }),
        Command::new("sim.despawn", vec!["grid_id"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            Ok(TermCmd::World(WorldDelta::DespawnGrid(grid_id)))
        }),
        Command::new("sim.ping", vec![], |_| Ok(TermCmd::Ping)),
        Command::new("sim.clear", vec![], |_| {
            Ok(TermCmd::World(WorldDelta::ClearAll))
        }),
        Command::new("sim.waypoint", vec!["grid_id", "x", "y"], |args| {
            let grid_id = Ent(parse_arg(args, "grid_id")?);
            let x = parse_arg(args, "x")?;
            let y = parse_arg(args, "y")?;
            Ok(TermCmd::World(WorldDelta::SetWaypoint {
                grid_id,
                waypoint: Isometry2d::from_xya(x, y, 0.0),
            }))
        }),
    ]
}

pub fn all_commands() -> Vec<Command<TermCmd>> {
    vec![
        Command::new("find", vec!["grid_name"], cmd_find),
        Command::new("speed", vec!["speed"], cmd_set_speed),
        Command::new("save", vec![], cmd_placeholder),
        Command::new("exit", vec![], cmd_exit),
        Command::new("say", vec!["msg"], cmd_say),
        Command::new("clear", vec![], cmd_clear),
        Command::new("ls.grids", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Grids))
        }),
        Command::new("ls.protos", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Protos))
        }),
        Command::new("ls.parts", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Parts))
        }),
        Command::new("ls.thrusters", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Thrusters))
        }),
        Command::new("ls.cpus", vec![], |_| {
            Ok(TermCmd::PrintEntityInfo(TableIdent::Computers))
        }),
        Command::new("server.disconnect", vec![], |_| {
            Ok(TermCmd::ServerDisconnect)
        }),
        Command::new("server.connect", vec![], |_| Ok(TermCmd::ServerConnect)),
        Command::new("server.info", vec![], |_| {
            Ok(TermCmd::RequestServerStatistics)
        }),
    ]
}
