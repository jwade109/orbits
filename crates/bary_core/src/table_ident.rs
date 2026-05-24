use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum TableIdent {
    Blueprints,
    Grids,
    Protos,
    Parts,
    Thrusters,
    Computers,
    Chunks,
    Tiles,
    Inventories,
    Machines,
}
