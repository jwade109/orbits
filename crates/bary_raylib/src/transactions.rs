use crate::sim::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_parts::*;
use early_returns::*;
use log::{info, warn};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WorldAction {
    Ping(Vec2),
    SpawnShipAt(String, Isometry2d),
    LoadWorld(World),
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
            _ = set_primary_computer_waypoint(grid_id, waypoint, world);
            _ = set_primary_computer_state(grid_id, true, world);
        }
        WorldAction::SetWaypointByName { name, waypoint } => {
            if let Some(grid_id) = get_grid_by_name(&world.grids, &name) {
                _ = set_primary_computer_waypoint(grid_id, waypoint, world);
                _ = set_primary_computer_state(grid_id, true, world);
                _ = toggle_tracking(world, grid_id);
            } else {
                warn!("Failed to find grid with name {name}");
            }
        }
        WorldAction::DespawnGrid(grid_id) => {
            _ = despawn_grid(world, grid_id);
        }
        WorldAction::InsertPart {
            grid_id,
            name,
            coord,
            rotation,
            layer,
        } => {
            let proto_id = some_or_return!(get_proto_by_name(&world.prototypes, &name));
            let proto = ok_or_return!(world.prototypes.try_get(proto_id));
            let region = GridRegion::new(coord, rotation, proto.dims);

            let instance = PartInstance {
                name,
                layer,
                region,
            };
            _ = insert_part(grid_id, world, &instance, true);
        }
        WorldAction::SetSpeed(speed) => {
            world.tick_rate = speed;
        }
        WorldAction::SetSourceItem {
            grid_id,
            coord,
            item,
        } => {
            let grid = ok_or_return!(world.grids.try_get(grid_id));
            let occ = grid.get_parts_at(coord).cloned().unwrap_or_default();
            let part_id = some_or_return!(occ.at_layer(PartLayer::Plumbing));
            let part = ok_or_return!(world.debug_portals.try_get_mut(part_id));
            if let PortalState::Source(old_item) = &mut part.state {
                *old_item = Some(item);
            }
        }
        WorldAction::InsertPipe { grid_id, src, dst } => {
            _ = insert_pipe(grid_id, src, dst, world);
        }
        WorldAction::SetRecipe {
            grid_id,
            coord,
            recipe,
        } => {
            let grid = ok_or_return!(world.grids.try_get(grid_id));
            let occ = grid.get_parts_at(coord).cloned().unwrap_or_default();
            let part_id = some_or_return!(occ.at_layer(PartLayer::Internal));
            let machine = ok_or_return!(world.machines.try_get_mut(part_id));
            machine.set_recipe(recipe);
        }
        WorldAction::SpawnAsteroid { iso, radius, seed } => {
            spawn_random_asteroid(world, iso, radius, seed);
        }
    }

    // WorldAction::SetCpuSelectedGrid(state) => {
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
