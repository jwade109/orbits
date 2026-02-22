use bary_core::prelude::*;

pub struct Part {
    pub placement: GridPlacement,
    pub prototype: EntityId,
    pub grid_id: EntityId,
}
