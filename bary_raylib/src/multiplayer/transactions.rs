use crate::systems::*;
use crate::world::World;
use bary_core::prelude::*;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum Action {
    Ping(Vec2),
    SpawnShipAt(String, Vec2),
    LoadWorld(World),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    tick: u64,
    action: Action,
}

impl Transaction {
    pub fn new(tick: u64, action: Action) -> Self {
        Self { tick, action }
    }
}

pub fn apply_transaction(world: &mut World, transaction: Transaction) {
    info!(
        "Applying transation {:?} at tick {}",
        transaction, world.ticks
    );
    match transaction.action {
        Action::Ping(pos) => {
            world::ping(world, pos);
        }
        Action::SpawnShipAt(name, pos) => {
            if let Ok(grid_id) = world::spawn_grid_by_name(world, &name) {
                _ = world::set_grid_position(world, grid_id, pos);
            }
        }
        Action::LoadWorld(new_world) => {
            *world = new_world;
        }
    }
}
