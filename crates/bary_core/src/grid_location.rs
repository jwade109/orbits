use crate::prelude::{Ent, PartCoord};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub struct GridLocation {
    pub grid_id: Ent,
    pub coord: PartCoord,
}

impl GridLocation {
    pub fn new(grid_id: Ent, coord: PartCoord) -> Self {
        Self { grid_id, coord }
    }
}
