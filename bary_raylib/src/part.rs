use bary_core::prelude::*;

pub struct Part {
    pub placement: GridPlacement,
    // TODO this field is kinda duplicated
    // with the prototype information.
    pub layer: PartLayer,
    pub prototype: Ent,
    pub grid_id: Ent,
}
