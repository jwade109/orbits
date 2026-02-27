use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Thruster {
    pub is_on: bool,
    pub is_rcs: bool,
    // TODO rename once formalized.
    pub thrust_millinewtons: i32,
    pub prototype: Ent,
    pub grid_id: Ent,
}
