use bary_core::prelude::*;
use bary_sim::World;
use log::*;
use std::path::{Path, PathBuf};

pub fn save_world(dir: impl AsRef<Path>, world: &World, overwrite: bool) -> BaryResult<()> {
    let dir = dir.as_ref();

    if std::fs::exists(dir)? {
        if overwrite {
            std::fs::remove_dir_all(dir)?;
        } else {
            return Err(BaryError::SaveAlreadyExists);
        }
    }

    std::fs::create_dir_all(dir)?;

    info!("Saving world to {}", dir.display());

    let s = bincode::serialize(&world.ticks).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("ticks.bin"), s)?;

    let s = bincode::serialize(&world.spawner).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("spawner.bin"), s)?;

    let s = bincode::serialize(&world.blueprints).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("blueprints.bin"), s)?;

    let s = bincode::serialize(&world.prototypes).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("prototypes.bin"), s)?;

    let s = bincode::serialize(&world.grids).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("grids.bin"), s)?;

    let s = bincode::serialize(&world.parts).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("parts.bin"), s)?;

    let s = bincode::serialize(&world.thrusters).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("thrusters.bin"), s)?;

    let s = bincode::serialize(&world.computers).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("computers.bin"), s)?;

    let s = bincode::serialize(&world.lights).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("lights.bin"), s)?;

    let s = bincode::serialize(&world.inventories).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("inventories.bin"), s)?;

    let s = bincode::serialize(&world.machines).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("machines.bin"), s)?;

    let s = bincode::serialize(&world.debug_portals).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("debug_portals.bin"), s)?;

    let s = bincode::serialize(&world.pipes).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("pipes.bin"), s)?;

    let s = bincode::serialize(&world.excavators).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("excavators.bin"), s)?;

    let s = bincode::serialize(&world.asteroids).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("asteroids.bin"), s)?;

    let s = bincode::serialize(&world.terrain_chunks).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("terrain_chunks.bin"), s)?;

    let s = bincode::serialize(&world.terrain_tiles).map_err(|_| BaryError::BincodeError)?;
    std::fs::write(dir.join("terrain_tiles.bin"), s)?;

    Ok(())
}

pub fn load_world(dir: impl AsRef<Path>) -> BaryResult<World> {
    let dir = dir.as_ref();
    info!("Loading world from {}", dir.display());

    let mut world = World::empty();

    let s = std::fs::read(dir.join("ticks.bin"))?;
    world.ticks = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("spawner.bin"))?;
    world.spawner = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("blueprints.bin"))?;
    world.blueprints = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("prototypes.bin"))?;
    world.prototypes = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("inventories.bin"))?;
    world.inventories = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("machines.bin"))?;
    world.machines = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("debug_portals.bin"))?;
    world.debug_portals = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("pipes.bin"))?;
    world.pipes = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("grids.bin"))?;
    world.grids = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("parts.bin"))?;
    world.parts = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("lights.bin"))?;
    world.lights = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("thrusters.bin"))?;
    world.thrusters = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("computers.bin"))?;
    world.computers = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("excavators.bin"))?;
    world.excavators = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("asteroids.bin"))?;
    world.asteroids = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("terrain_chunks.bin"))?;
    world.terrain_chunks = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    let s = std::fs::read(dir.join("terrain_tiles.bin"))?;
    world.terrain_tiles = bincode::deserialize(&s).map_err(|_| BaryError::BincodeError)?;

    Ok(world)
}

pub fn list_saves_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut ret = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            if let Ok(e) = entry {
                ret.push(e.path());
            }
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use crate::{sim::update_world, world_builder::WorldBuilder};

    use super::*;

    #[test]
    fn world_persistence() {
        let save_path = "../../saves/test_world";

        if std::fs::exists(save_path).unwrap() {
            std::fs::remove_dir_all(save_path).unwrap();
        }

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("remora")
            .blueprint("bellerophon")
            .blueprint("foundation")
            .spawn("pollux", "billy", (120.0, 43.0, 0.4))
            .spawn("remora", "sally", (-30.0, 21.0, -0.1))
            .spawn("bellerophon", "eisenhower", (50.0, 109.0, 1.4))
            .waypoint("pollux", (50.0, 300.0, 0.2))
            .build();

        for _ in 0..1000 {
            update_world(&mut world);
        }

        let r = save_world(save_path, &world, false);
        assert_eq!(r, Ok(()));

        let r = save_world(save_path, &world, false);
        assert_eq!(r, Err(BaryError::SaveAlreadyExists));

        let r = save_world(save_path, &world, true);
        assert_eq!(r, Ok(()));

        let world = load_world(save_path).expect("Expected successful load");

        assert_eq!(world.grids.len(), 3);
        assert_eq!(world.parts.len(), 374);
    }
}
