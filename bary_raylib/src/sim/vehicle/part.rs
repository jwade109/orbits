use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Part {
    pub region: GridRegion,
    // TODO this field is kinda duplicated
    // with the prototype information.
    pub mass: Mass,
    pub layer: PartLayer,
    pub prototype: Ent,
    pub grid_id: Ent,
    pub classification: PartClassification,
}
