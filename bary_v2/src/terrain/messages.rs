use bevy::prelude::{Entity, Event, IVec2, UVec2, Vec2};
use bary_core::prelude::*;

#[derive(Event, Debug, Clone, Copy)]
pub struct GenerateChunk {
    pub pos: IVec2,
    pub material: Option<Item>,
    pub log: bool,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct DeleteChunk {
    pub pos: IVec2,
    pub log: bool,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct Excavate {
    pub pos: Vec2,
    pub radius: f32,
    pub is_fill: bool,
}

#[derive(Event, Debug, Clone, Copy)]
pub struct MineToInventory {
    pub inventory: Entity,
    pub pos: Vec2,
}
