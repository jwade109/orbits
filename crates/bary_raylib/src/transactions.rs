use bary_core::prelude::*;
use bary_sim::WorldDelta;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TermCmd {
    World(WorldDelta),
    Exit,
    Say(String),
    Ping,
    Clear,
    SetSimSpeed(u32),
    FindGridByName(String),
    ListGrids,
    ListProtos,
    ListParts,
    ListThrusters,
    ListComputers,
    ServerDisconnect,
    ServerConnect,
    RequestServerStatistics,
    SetWaypoint(Ent, Isometry2d),
    EchoSaveInfo,
    LoadSave(String),
    SaveWorldToDisk(String, bool),
    ListSaves,
    ListBlueprints,
}
