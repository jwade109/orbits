use bary_core::prelude::*;

pub struct Thruster {
    pub is_on: bool,
    // TODO rename once formalized.
    pub thrust_millinewtons: i32,
    pub prototype: EntityId,
    pub grid_id: EntityId,
}
