use crate::client::ClientSpecificInfo;
use crate::ops;
use crate::sim::world::World;
use crate::{sim::systems::*, sim::world::update_world};
use bary_core::prelude::*;
use log::{info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WorldAction {
    Ping(Vec2),
    SpawnShipAt(String, Isometry2d),
    LoadWorld(World),
    FastForwardTo(u64),
    SetWaypoint { grid_id: Ent, waypoint: Isometry2d },
    SetWaypointByName { name: String, waypoint: Isometry2d },
    DespawnGrid(Ent),
    SetSpeed(u32),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ClientAction {
    SetCpuSelectedGrid(bool),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Action {
    World(WorldAction),
    Client(ClientAction),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction {
    pub tick: u64,
    pub action: Action,
}

impl Transaction {
    pub fn new(tick: u64, action: Action) -> Self {
        Self { tick, action }
    }
}

pub fn apply_action(world: &mut World, client: &mut ClientSpecificInfo, action: Action) {
    if let Action::World(action) = action {
        apply_world_action(world, action);
    }
}

pub fn apply_world_action(world: &mut World, action: WorldAction) {
    info!("Applying action {:?} at tick {}", action, world.ticks);
    match action {
        WorldAction::Ping(pos) => {
            ping(world, pos);
        }
        WorldAction::SpawnShipAt(bp_name, iso) => {
            if let Ok(grid_id) = spawn_grid_with_random_name(world, bp_name) {
                _ = set_grid_pose(world, grid_id, iso);
            }
        }
        WorldAction::LoadWorld(new_world) => {
            *world = new_world;
        }
        WorldAction::FastForwardTo(tick) => {
            if world.ticks < tick {
                let delta = tick - world.ticks;
                warn!("Fast forwarding by {} ticks", delta);
                while world.ticks < tick {
                    update_world(world);
                }
            } else if tick < world.ticks {
                let delta = world.ticks - tick;
                warn!("Already ahead of fast forward directive by {} ticks", delta);
            }
        }
        WorldAction::SetWaypoint { grid_id, waypoint } => {
            _ = ops::set_primary_computer_waypoint(grid_id, waypoint, world);
            _ = ops::set_primary_computer_state(grid_id, true, world);
        }
        WorldAction::SetWaypointByName { name, waypoint } => {
            if let Some(grid_id) = find::grid_by_name(&world.grids, &name) {
                _ = set_primary_computer_waypoint(grid_id, waypoint, world);
                _ = set_primary_computer_state(grid_id, true, world);
                _ = toggle_tracking(world, grid_id);
            } else {
                warn!("Failed to find grid with name {name}");
            }
        }
        WorldAction::DespawnGrid(grid_id) => {
            _ = ops::despawn_grid(world, grid_id);
        }
        WorldAction::SetSpeed(speed) => {
            world.tick_rate = speed;
        } // WorldAction::SetCpuSelectedGrid(state) => {
          //     let grid_id = match &client.viewport {
          //         crate::client::Viewport::Editor(e) => Some(e.vehicle),
          //         crate::client::Viewport::Free(f) => {
          //             f.selection_info.selected.first().map(|e| e.grid_id)
          //         }
          //     };

          //     if let Some(grid_id) = grid_id {
          //         _ = ops::set_primary_computer_state(grid_id, state, world);
          //         _ = ops::set_all_thrusters(grid_id, false, world);
          //         client
          //             .chat
          //             .log(format!("Set CPU on grid {} to {}", grid_id, state));
          //     } else {
          //         client.chat.log("No grid selected");
          //     }
          // }
    }
}
