use enum_iterator::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Sequence)]
pub enum TableIdent {
    Blueprints,
    Grids,
    Protos,
    Parts,
    Thrusters,
    Computers,
    Asteroids,
    Chunks,
    Tiles,
    Inventories,
    Machines,
    Pipes,
    Lights,
    Excavators,
}

impl TableIdent {
    pub fn all() -> impl Iterator<Item = Self> {
        all::<Self>()
    }
}

impl std::fmt::Display for TableIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
