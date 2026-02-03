use crate::prelude::*;
use bevy::math::IVec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlotData {
    pub filter: ItemFilter,
    pub volume_liters: f32,
    pub min: IVec2,
    pub max: IVec2,
    pub name: Option<String>,
    pub is_fluid: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InventoryData {
    pub slots: Vec<SlotData>,
}
