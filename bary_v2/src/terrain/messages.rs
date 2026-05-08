use bary_core::prelude::*;
use bevy::prelude::{Entity, IVec2, Message, UVec2, Vec2};

#[derive(Message, Debug, Clone, Copy)]
pub struct GenerateChunk {
    pub pos: IVec2,
    pub material: Option<Item>,
    pub log: bool,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct DeleteChunk {
    pub pos: IVec2,
    pub log: bool,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct Excavate {
    pub pos: Vec2,
    pub radius: f32,
    pub is_fill: bool,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct MineToInventory {
    pub inventory: Entity,
    pub pos: Vec2,
}
