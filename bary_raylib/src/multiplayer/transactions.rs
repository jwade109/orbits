use crate::systems::*;
use crate::world::World;
use bary_core::prelude::*;
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Transaction {
    Ping(Vec2),
    SpawnShip(String),
}

pub fn apply_transaction(world: &mut World, transaction: Transaction) {
    info!(
        "Applying transation {:?} at tick {}",
        transaction, world.ticks
    );
    match transaction {
        Transaction::Ping(pos) => {
            world::ping(world, pos);
        }
        Transaction::SpawnShip(name) => {
            if let Ok(grid_id) = world::spawn_grid_by_name(world, &name) {
                let pos = randvec(10.0, 200.0);
                _ = world::set_grid_position(world, grid_id, pos);
            }
        }
    }
}
