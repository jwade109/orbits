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

impl std::fmt::Display for TableIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
