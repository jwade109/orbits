use crate::starling::factory::ItemFilter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InventoryData {
    pub slots: usize,
    pub filter: ItemFilter,
    pub volume_liters: f32,
    pub is_fuel: bool,
}
