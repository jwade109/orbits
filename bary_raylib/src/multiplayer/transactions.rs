use crate::client::ClientSpecificInfo;
use crate::sim::world::World;
use crate::{ops, query};
use crate::{sim::systems::*, sim::world::update_world};
use bary_core::prelude::*;
use log::{info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Action {
    Ping(Vec2),
    SpawnShipAt(String, Isometry2d),
    LoadWorld(World),
    FastForwardTo(u64),
    SetWaypoint { grid_id: Ent, waypoint: Isometry2d },
    LookAt(String),
    DespawnGrid(Ent),
    ClearWorld,
    SetSpeed(u32),
    SetCpuSelectedGrid(bool),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction {
    tick: u64,
    action: Action,
}

impl Transaction {
    pub fn new(tick: u64, action: Action) -> Self {
        Self { tick, action }
    }
}

pub fn apply_transaction(
    world: &mut World,
    client: &mut ClientSpecificInfo,
    transaction: Transaction,
) {
    info!(
        "Applying transation {:?} at tick {}",
        transaction, world.ticks
    );
    match transaction.action {
        Action::Ping(pos) => {
            if world.ticks < transaction.tick {
                let delta = transaction.tick - world.ticks;
                warn!("Ping is ahead by {} ticks", delta);
            } else if transaction.tick < world.ticks {
                let delta = world.ticks - transaction.tick;
                warn!("Ping is late by {} ticks", delta);
            }
            ping(world, pos);
        }
        Action::SpawnShipAt(name, iso) => {
            if let Ok(grid_id) = spawn_grid_by_name(world, &name) {
                _ = set_grid_pose(world, grid_id, iso);
            }
        }
        Action::LoadWorld(new_world) => {
            *world = new_world;
        }
        Action::FastForwardTo(tick) => {
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
        Action::SetWaypoint { grid_id, waypoint } => {
            _ = ops::set_primary_computer_waypoint(grid_id, waypoint, world);
            _ = ops::set_primary_computer_state(grid_id, true, world);
        }
        Action::LookAt(name) => {
            if let Some(grid_id) = query::grid_by_name(&world.grids, &name) {
                client.viewport.look_at(grid_id);
                world.target_camera.zoom = 15.0;
            }
        }
        Action::ClearWorld => {
            ops::despawn_all_vehicles(world);
        }
        Action::DespawnGrid(grid_id) => {
            _ = ops::despawn_grid(world, grid_id);
        }
        Action::SetSpeed(speed) => {
            world.tick_rate = speed;
            let s = format!("Set tick rate to {}", speed);
            client.chat.log(s);
        }
        Action::SetCpuSelectedGrid(state) => {
            let grid_id = match &client.viewport {
                crate::client::Viewport::Editor(e) => Some(e.vehicle),
                crate::client::Viewport::Free(f) => {
                    f.selection_info.selected.first().map(|e| e.grid_id)
                }
            };

            if let Some(grid_id) = grid_id {
                _ = ops::set_primary_computer_state(grid_id, state, world);
                _ = ops::set_all_thrusters(grid_id, false, world);
                client
                    .chat
                    .log(format!("Set CPU on grid {} to {}", grid_id, state));
            } else {
                client.chat.log("No grid selected");
            }
        }
    }
}
