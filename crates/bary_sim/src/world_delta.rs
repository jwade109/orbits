use bary_core::prelude::*;
use bary_factory::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WorldDelta {
    Ping(Vec2),
    SpawnShipAt(String, Isometry2d),
    FastForwardTo(u64),
    SetWaypoint {
        grid_id: Ent,
        waypoint: Isometry2d,
    },
    SetWaypointByName {
        name: String,
        waypoint: Isometry2d,
    },
    DespawnGrid(Ent),
    SetSpeed(u32),
    InsertPart {
        grid_id: Ent,
        name: String,
        coord: PartCoord,
        rotation: Rotation,
        layer: PartLayer,
    },
    SetSourceItem {
        grid_id: Ent,
        coord: PartCoord,
        item: Item,
    },
    InsertPipe {
        grid_id: Ent,
        src: PartCoord,
        dst: PartCoord,
    },
    SetRecipe {
        grid_id: Ent,
        coord: PartCoord,
        recipe: RecipeListing,
    },
    SpawnAsteroid {
        iso: Isometry2d,
        radius: f32,
        seed: u64,
    },
}
