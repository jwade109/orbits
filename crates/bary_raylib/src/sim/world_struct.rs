use crate::sim::*;
use bary_core::prelude::*;
use bary_factory::*;
use bary_parts::*;
use bary_sim::*;
use log::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct World {
    pub ticks: u64,

    // debug info
    pub grid_acceleration_updates: u64,

    // components - to be synchronized between clients
    pub spawner: EntitySpawner,
    pub particles: Vec<PingParticle>,
    pub blueprints: Components<NamedBlueprint>,
    pub prototypes: Components<PartPrototype>,
    pub parts: Components<Part>,
    pub thrusters: Components<Thruster>,
    pub computers: Components<Computer>,
    pub lights: Components<Light>,
    pub grids: Components<VehicleGrid>,
    pub tracking: Components<Tracker>,
    pub inventories: Components<Inventory>,
    pub machines: Components<Machine>,
    pub stars: Components<Star>,
    pub pipes: Components<Pipe>,
    pub debug_portals: Components<DebugPortal>,
    pub asteroids: Components<BigRock>,

    pub terrain_chunks: Components<TerrainChunk>,
    pub terrain_tiles: Components<TerrainTile>,

    // TODO might move this to assets.
    pub ship_names: Vec<String>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "World({}, {} grids, {})",
            self.ticks,
            self.grids.len(),
            self.spawner.next()
        )
    }
}

impl World {
    pub fn empty() -> Self {
        Self {
            ship_names: vec![
                "Gary".to_string(),
                "Sally".to_string(),
                "Juliet".to_string(),
                "Violet".to_string(),
                "Charlie".to_string(),
                "Orville".to_string(),
            ],
            ..Default::default()
        }
    }

    #[must_use]
    pub fn apply(&mut self, delta: WorldDelta) -> BaryResult<()> {
        info!("Applying delta {:?} at tick {}", delta, self.ticks);
        match delta {
            WorldDelta::ClearAll => {
                *self = World::empty();
                Ok(())
            }
            WorldDelta::Ping(pos) => {
                ping(self, pos);
                Ok(())
            }
            WorldDelta::SpawnShipAt(bp_name, iso) => {
                let grid_id = spawn_grid_with_random_name(self, bp_name)?;
                set_grid_pose(self, grid_id, iso)?;
                Ok(())
            }
            WorldDelta::FastForwardTo(tick) => {
                if self.ticks < tick {
                    let delta = tick - self.ticks;
                    warn!("Fast forwarding by {} ticks", delta);
                    while self.ticks < tick {
                        update_world(self);
                    }
                } else if tick < self.ticks {
                    let delta = self.ticks - tick;
                    warn!("Already ahead of fast forward directive by {} ticks", delta);
                }
                Ok(())
            }
            WorldDelta::SetWaypoint { grid_id, waypoint } => {
                _ = set_primary_computer_waypoint(grid_id, waypoint, self);
                _ = set_primary_computer_state(grid_id, true, self);
                Ok(())
            }
            WorldDelta::SetWaypointByName { name, waypoint } => {
                if let Some(grid_id) = get_grid_by_name(&self.grids, &name) {
                    _ = set_primary_computer_waypoint(grid_id, waypoint, self);
                    _ = set_primary_computer_state(grid_id, true, self);
                    _ = toggle_tracking(self, grid_id);
                } else {
                    warn!("Failed to find grid with name {name}");
                }
                Ok(())
            }
            WorldDelta::DespawnGrid(grid_id) => {
                _ = despawn_grid(self, grid_id);
                Ok(())
            }
            WorldDelta::InsertPart {
                grid_id,
                name,
                coord,
                rotation,
                layer,
            } => {
                let proto_id = get_proto_by_name(&self.prototypes, &name)
                    .ok_or(BaryError::NoProtoWithName(name.clone()))?;
                let proto = self.prototypes.try_get(proto_id)?;
                let region = GridRegion::new(coord, rotation, proto.dims);

                let instance = PartInstance {
                    name,
                    layer,
                    region,
                };
                _ = insert_part(grid_id, self, &instance, true);
                Ok(())
            }
            WorldDelta::SetSourceItem {
                grid_id,
                coord,
                item,
            } => {
                let grid = self.grids.try_get(grid_id)?;
                let occ = grid.get_parts_at(coord).cloned().unwrap_or_default();
                let part_id = occ
                    .at_layer(PartLayer::Plumbing)
                    .ok_or(BaryError::NoPartsInLayer(PartLayer::Plumbing))?;
                let part = self.debug_portals.try_get_mut(part_id)?;
                if let PortalState::Source(old_item) = &mut part.state {
                    *old_item = Some(item);
                }
                Ok(())
            }
            WorldDelta::InsertPipe { grid_id, src, dst } => {
                _ = insert_pipe(grid_id, src, dst, self);
                Ok(())
            }
            WorldDelta::SetRecipe {
                grid_id,
                coord,
                recipe,
            } => {
                let grid = self.grids.try_get(grid_id)?;
                let occ = grid.get_parts_at(coord).cloned().unwrap_or_default();
                let part_id = occ
                    .at_layer(PartLayer::Internal)
                    .ok_or(BaryError::NoPartsInLayer(PartLayer::Internal))?;
                let machine = self.machines.try_get_mut(part_id)?;
                machine.set_recipe(recipe);
                Ok(())
            }
            WorldDelta::SpawnAsteroid { iso, radius, seed } => {
                spawn_random_asteroid(self, iso, radius, seed);
                Ok(())
            }
        }
    }
}
