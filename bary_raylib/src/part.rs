use bary_core::prelude::*;

pub struct Part {
    pub placement: GridPlacement,
    // TODO this field is kinda duplicated
    // with the prototype information.
    pub layer: PartLayer,
    pub prototype: EntityId,
    pub grid_id: EntityId,
}
