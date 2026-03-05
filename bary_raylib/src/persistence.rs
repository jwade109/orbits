use crate::result::*;
use crate::world::World;
use bary_core::prelude::*;
use log::*;
use std::path::Path;

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
    let blueprints_dir = dir.join("blueprints");
    let grids_path = dir.join("grids.toml");
    let parts_path = dir.join("parts.toml");
    let thrusters_path = dir.join("thrusters.toml");
    let computers_path = dir.join("computers.toml");
    let lights_path = dir.join("lights.toml");

    std::fs::create_dir(&blueprints_dir)?;

    for (name, bp) in world.blueprints.values() {
        let path = blueprints_dir.join(format!("{}.bp", name));
        if let Err(e) = save_vehicle(path, bp) {
            error!("Failed to save blueprint: {}", e);
            return Err(BaryError::FailedToSaveBlueprint);
        }
    }

    let s = toml::to_string(&world.grids)?;
    std::fs::write(grids_path, s)?;

    let s = toml::to_string(&world.parts)?;
    std::fs::write(parts_path, s)?;

    let s = toml::to_string(&world.thrusters)?;
    std::fs::write(thrusters_path, s)?;

    let s = toml::to_string(&world.computers)?;
    std::fs::write(computers_path, s)?;

    let s = toml::to_string(&world.lights)?;
    std::fs::write(lights_path, s)?;

    Ok(())
}

pub fn load_world(dir: impl AsRef<Path>) -> BaryResult<World> {
    let dir = dir.as_ref();
    info!("Loading world from {}", dir.display());

    let grids_path = dir.join("grids.toml");
    let parts_path = dir.join("parts.toml");
    let thrusters_path = dir.join("thrusters.toml");
    let computers_path = dir.join("computers.toml");
    let lights_path = dir.join("lights.toml");

    let mut world = World::empty();

    let s = std::fs::read_to_string(grids_path)?;
    world.grids = toml::from_str(&s)?;

    let s = std::fs::read_to_string(parts_path)?;
    world.parts = toml::from_str(&s)?;

    let s = std::fs::read_to_string(lights_path)?;
    world.lights = toml::from_str(&s)?;

    let s = std::fs::read_to_string(thrusters_path)?;
    world.thrusters = toml::from_str(&s)?;

    let s = std::fs::read_to_string(computers_path)?;
    world.computers = toml::from_str(&s)?;

    Ok(world)
}

#[cfg(test)]
mod tests {
    use crate::{input_state::InputState, world::update_world, world_builder::WorldBuilder};

    use super::*;

    #[test]
    fn world_persistence() {
        let save_path = "../saves/test_world";

        if std::fs::exists(save_path).unwrap() {
            std::fs::remove_dir_all(save_path).unwrap();
        }

        let mut world = WorldBuilder::new()
            .test_assets()
            .blueprint("pollux")
            .blueprint("remora")
            .blueprint("bellerophon")
            .blueprint("foundation")
            .spawn("pollux", (120.0, 43.0, 0.4))
            .spawn("remora", (-30.0, 21.0, -0.1))
            .spawn("bellerophon", (50.0, 109.0, 1.4))
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
