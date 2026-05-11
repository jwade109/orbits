use bary_core::prelude::*;
use bary_factory::ItemFilter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SlotPrototype {
    pub filter: ItemFilter,
    pub volume_liters: f32,
    pub min: IVec2,
    pub max: IVec2,
    pub name: Option<String>,
    pub is_fluid: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InventoryPrototype {
    pub slots: Vec<SlotPrototype>,
}
