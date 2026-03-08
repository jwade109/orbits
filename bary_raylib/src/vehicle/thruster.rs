use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Thruster {
    pub is_on: bool,
    pub is_rcs: bool,
    pub thrust: f32,
    pub prototype: Ent,
    pub grid_id: Ent,
    /// the computer that last modified this thruster's state
    pub last_controlled_by: Option<Ent>,
}
