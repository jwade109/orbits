use bary_core::prelude::*;
use bevy::prelude::*;

use crate::SelectedPointInfo;

#[derive(Event, Debug)]
pub struct HoldHereCommand(pub Entity);

#[derive(Event, Debug)]
pub enum SpacecraftEvent {
    SpawnVehicle {
        ship_name: String,
        blueprint_name: String,
        pos: Vec2,
        angle: f32,
    },
    SpawnPart {
        name: String,
        pos: Vec2,
        angle: f32,
    },
    Destroy {
        target: Entity,
    },
}

#[derive(Event, Debug, Clone, Copy)]
pub struct DockingTrigger {
    pub chief: Entity,
    pub deputy: Entity,
    pub offset: PartCoord,
    pub rotation: Rotation,
}

#[derive(Event, Debug)]
pub struct PositionHoldCommand {
    pub pos: Vec2,
    pub angle: f32,
}

#[derive(Event, Debug)]
pub struct DeltaVelocity {
    pub target_grid: Entity,
    pub delta_velocity: Vec2,
}

#[derive(Event, Debug)]
pub struct AddHose {
    pub start: (Entity, PartCoord),
    pub end: (Entity, PartCoord),
}
