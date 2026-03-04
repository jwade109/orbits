use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Part {
    pub placement: GridPlacement,
    // TODO this field is kinda duplicated
    // with the prototype information.
    pub layer: PartLayer,
    pub prototype: Ent,
    pub grid_id: Ent,
    pub classification: PartClassification,
}
