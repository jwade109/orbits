use crate::result::*;
use crate::systems::*;
use crate::world::World;
use bary_core::prelude::*;
use log::*;
use std::path::{Path, PathBuf};

pub fn save_world(dir: impl AsRef<Path>, world: &World, overwrite: bool) -> BaryResult<()> {
    let dir = dir.as_ref();

    std::fs::create_dir_all(dir)?;

    info!("Saving world to {}", dir.display());
    let blueprints_dir = dir.join("blueprints");
    let vehicles_dir = dir.join("vehicles");
    let thrusters_dir = dir.join("thrusters");

    std::fs::create_dir(&blueprints_dir)?;

    for (name, bp) in world.blueprints.values() {
        let path = blueprints_dir.join(format!("{}.bp", name));
        if let Err(e) = save_vehicle(path, bp) {
            error!("Failed to save blueprint: {}", e);
            return Err(BaryError::FailedToSaveBlueprint);
        }
    }

    std::fs::create_dir(&vehicles_dir)?;

    for (grid_id, grid) in world.grids.iter() {
        let name = format!("{}.grid", grid_id);
        let filepath = vehicles_dir.join(name);
        let s = toml::to_string(grid)?;
        std::fs::write(filepath, s)?;
    }

    std::fs::create_dir(&thrusters_dir)?;

    for (thruster_id, thruster) in world.thrusters.iter() {
        let name = format!("{}.thruster", thruster_id);
        let filepath = thrusters_dir.join(name);
        let s = toml::to_string(thruster)?;
        std::fs::write(filepath, s)?;
    }

    Ok(())
}

pub fn load_world(dir: impl AsRef<Path>) -> BaryResult<World> {
    let dir = dir.as_ref();
    info!("Loading world from {}", dir.display());
    Ok(World::empty())
}

#[cfg(test)]
mod tests {
    use crate::world_builder::WorldBuilder;

    use super::*;

    #[test]
    fn world_persistence() {
        let save_path = "../saves/test_world";

        if std::fs::exists(save_path).unwrap() {
            std::fs::remove_dir_all(save_path).unwrap();
        }

        let world = WorldBuilder::new()
            .assets("../assets/")
            .blueprint("pollux")
            .blueprint("remora")
            .blueprint("bellerophon")
            .blueprint("foundation")
            .spawn("pollux", (120.0, 43.0, 0.4))
            .spawn("remora", (-30.0, 21.0, -0.1))
            .build();

        let r = save_world(save_path, &world, true);
        assert_eq!(r, Ok(()));

        let loaded = load_world(save_path).unwrap();
    }
}
